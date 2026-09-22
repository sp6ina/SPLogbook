// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::geo::{
    calculate_solar_position, locator_to_coordinates, Coordinates,
};
use crate::core::i18n::tr;
use crate::gui::app::SpLogApp;
use eframe::egui;

// Kontury kontynentów (Współrzędne geograficzne w stopniach: [Szerokość Lat, Długość Lon])
const CONTINENT_EUROPE: &[(f64, f64)] = &[
    (71.0, 28.0), (70.0, 20.0), (62.0, 5.0), (58.0, 6.0), (55.0, 9.0),
    (54.0, 8.0), (51.0, 2.0), (48.0, -4.5), (43.5, -9.0), (37.0, -9.0),
    (36.0, -5.5), (38.0, 0.0), (43.0, 3.0), (43.5, 10.0), (41.0, 16.0),
    (38.0, 15.0), (36.5, 22.0), (40.0, 24.0), (41.0, 29.0), (44.0, 29.0),
    (46.0, 31.0), (45.0, 36.0), (42.0, 42.0), (41.0, 48.0), (46.0, 48.0),
    (54.0, 40.0), (60.0, 30.0), (68.0, 31.0), (71.0, 28.0),
];

const BRITISH_ISLES: &[(f64, f64)] = &[
    (58.5, -3.0), (58.0, -5.0), (55.0, -6.0), (50.0, -5.0), (51.0, 1.5),
    (53.0, 0.0), (55.0, -1.5), (58.5, -3.0),
];

const IRELAND: &[(f64, f64)] = &[
    (55.0, -7.0), (51.5, -10.0), (52.0, -6.0), (54.5, -6.0), (55.0, -7.0),
];

const CONTINENT_ASIA: &[(f64, f64)] = &[
    (70.0, 30.0), (73.0, 70.0), (73.0, 80.0), (77.0, 105.0), (70.0, 179.0),
    (66.0, 170.0), (60.0, 163.0), (52.0, 143.0), (43.0, 132.0), (38.0, 128.0),
    (35.0, 129.0), (31.0, 122.0), (22.0, 114.0), (10.0, 107.0), (1.0, 104.0),
    (13.0, 100.0), (21.0, 90.0), (8.0, 77.0), (23.0, 68.0), (25.0, 57.0),
    (13.0, 44.0), (28.0, 34.0), (31.0, 35.0), (37.0, 36.0), (41.0, 29.0),
    (46.0, 48.0), (55.0, 60.0), (68.0, 66.0), (70.0, 30.0),
];

const JAPAN: &[(f64, f64)] = &[
    (45.0, 142.0), (43.0, 145.0), (35.0, 140.0), (33.0, 131.0),
    (37.0, 137.0), (41.0, 141.0), (45.0, 142.0),
];

const CONTINENT_AFRICA: &[(f64, f64)] = &[
    (36.0, -5.5), (37.0, 10.0), (32.0, 20.0), (31.0, 32.0), (28.0, 34.0),
    (12.0, 44.0), (11.0, 51.0), (-1.0, 42.0), (-12.0, 40.0), (-26.0, 33.0),
    (-34.0, 18.0), (-30.0, 17.0), (-16.0, 12.0), (-5.0, 12.0), (4.0, 9.0),
    (6.0, 2.0), (5.0, -4.0), (15.0, -17.0), (21.0, -17.0), (28.0, -13.0),
    (36.0, -5.5),
];

const MADAGASCAR: &[(f64, f64)] = &[
    (-12.0, 49.0), (-16.0, 50.0), (-25.0, 47.0), (-25.0, 43.0),
    (-16.0, 44.0), (-12.0, 49.0),
];

const CONTINENT_NORTH_AMERICA: &[(f64, f64)] = &[
    (71.0, -156.0), (70.0, -135.0), (69.0, -100.0), (60.0, -94.0),
    (51.0, -80.0), (58.0, -63.0), (47.0, -53.0), (44.0, -66.0),
    (35.0, -75.0), (25.0, -80.0), (29.0, -89.0), (26.0, -97.0),
    (20.0, -97.0), (16.0, -93.0), (9.0, -83.0), (7.0, -78.0),
    (14.0, -92.0), (19.0, -105.0), (32.0, -117.0), (37.0, -122.0),
    (48.0, -125.0), (58.0, -136.0), (60.0, -148.0), (55.0, -165.0),
    (65.0, -168.0), (71.0, -156.0),
];

const GREENLAND: &[(f64, f64)] = &[
    (83.0, -30.0), (81.0, -18.0), (76.0, -19.0), (70.0, -22.0),
    (60.0, -43.0), (65.0, -53.0), (70.0, -54.0), (76.0, -68.0),
    (82.0, -60.0), (83.0, -30.0),
];

const CONTINENT_SOUTH_AMERICA: &[(f64, f64)] = &[
    (12.0, -72.0), (10.0, -62.0), (6.0, -55.0), (-3.0, -39.0),
    (-5.0, -35.0), (-13.0, -39.0), (-23.0, -43.0), (-34.0, -54.0),
    (-40.0, -62.0), (-54.0, -68.0), (-53.0, -74.0), (-40.0, -74.0),
    (-18.0, -70.0), (-5.0, -81.0), (1.0, -79.0), (8.0, -77.0),
    (12.0, -72.0),
];

const CONTINENT_AUSTRALIA: &[(f64, f64)] = &[
    (-12.0, 132.0), (-12.0, 137.0), (-17.0, 139.0), (-11.0, 142.0),
    (-25.0, 153.0), (-37.0, 150.0), (-38.0, 145.0), (-35.0, 137.0),
    (-32.0, 133.0), (-35.0, 118.0), (-32.0, 115.0), (-22.0, 114.0),
    (-16.0, 123.0), (-12.0, 132.0),
];

const NEW_ZEALAND: &[(f64, f64)] = &[
    (-35.0, 174.0), (-37.0, 178.0), (-41.0, 175.0), (-46.0, 168.0),
    (-44.0, 168.0), (-41.0, 172.0), (-35.0, 174.0),
];

const CONTINENT_ANTARCTICA: &[(f64, f64)] = &[
    (-64.0, -60.0), (-65.0, -64.0), (-73.0, -100.0), (-78.0, -160.0),
    (-85.0, 180.0), (-72.0, 170.0), (-66.0, 140.0), (-66.0, 110.0),
    (-66.0, 90.0), (-69.0, 70.0), (-70.0, 40.0), (-70.0, 10.0),
    (-70.0, -10.0), (-75.0, -30.0), (-64.0, -60.0),
];

#[derive(Clone, Copy, Debug)]
struct MapState {
    zoom: f32,
    pan: egui::Vec2,
}
impl Default for MapState {
    fn default() -> Self {
        Self { zoom: 1.0, pan: egui::Vec2::ZERO }
    }
}

pub fn render_world_map_tile(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;
    ui.group(|ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("🗺 {}", tr("map.world_title", lang))).strong().size(12.0).color(egui::Color32::from_rgb(56, 189, 248)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕").on_hover_text(tr("window.hide_tooltip", lang)).clicked() {
                        app.panel_world_map.visible = false;
                        app.show_world_map_window = false;
                        app.save_station_config();
                    }
                    if ui.button("↗").on_hover_text("Odepnij do osobnego okna systemu (multi-monitor)").clicked() {
                        app.panel_world_map.floating = true;
                        app.show_world_map_window = true;
                        app.save_station_config();
                    }
                });
            });
            ui.separator();
            render_world_map_content(app, ui);
        });
    });
}

/// Okno interaktywnej mapy świata z wizualizacją linii zmierzchu (Grayline) i ortodromy
pub fn render_world_map_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.panel_world_map.visible {
        return;
    }

    let lang = app.current_language;

    // Obsługa Multi-Viewport (niezależne okno OS)
    if app.panel_world_map.floating {
        let mut still_open = true;
        let mut dock_back = false;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("world_map_viewport"),
            egui::ViewportBuilder::default()
                .with_title(format!("🗺 {} - SPLogbook", tr("map.world_title", lang)))
                .with_inner_size([900.0, 560.0])
                .with_min_inner_size([460.0, 300.0]),
            |ctx, _class| {
                egui::TopBottomPanel::top("world_map_vp_bar").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button(format!("↙ {}", tr("window.dock", lang))).on_hover_text(tr("window.dock_tooltip", lang)).clicked() {
                            dock_back = true;
                        }
                    });
                });
                egui::CentralPanel::default().show(ctx, |ui| {
                    render_world_map_content(app, ui);
                });
                if ctx.input(|i| i.viewport().close_requested()) {
                    still_open = false;
                }
            },
        );

        if dock_back {
            app.panel_world_map.floating = false;
            app.save_station_config();
        }
        if !still_open {
            app.panel_world_map.visible = false;
            app.panel_world_map.floating = false;
            app.save_station_config();
        }
        return;
    }

    let mut is_open = app.panel_world_map.visible;
    let screen = ctx.available_rect();
    let right_w = 360.0_f32.min((screen.width() - 500.0).max(200.0));
    let mid_w = (screen.width() - 488.0 - right_w - 20.0).max(380.0);

    let default_pos = [screen.min.x + 488.0, screen.min.y + 8.0];
    let default_size = [mid_w, (screen.height() - 20.0).max(450.0)];

    let mut win = egui::Window::new(egui::RichText::new(format!("   🗺 {}", tr("map.world_title", lang))).size(12.0).strong())
        .open(&mut is_open)
        .min_size([400.0, 300.0])
        .resizable(true)
        .collapsible(true)
        .constrain_to(screen);

    if app.reset_layout_requested {
        win = win.current_pos(default_pos).default_size(default_size);
    } else if let Some(pos) = app.panel_world_map.saved_pos {
        let sz = app.panel_world_map.saved_size.unwrap_or(default_size);
        win = win.current_pos(pos).default_size(sz);
    } else {
        win = win.default_pos(default_pos).default_size(default_size);
    }

    let win_res = win.show(ctx, |ui| {
        render_world_map_content(app, ui);
    });

    if let Some(ref res) = win_res {
        crate::gui::render_titlebar_popout_button_if(ctx, "world_map_popout_btn", res.response.layer_id, res.response.rect, &mut app.panel_world_map.floating, true);
        if app.panel_world_map.floating {
            app.save_station_config();
        }
        if res.response.dragged() || res.response.drag_stopped() {
            let r = res.response.rect;
            let new_pos = [r.min.x, r.min.y];
            let new_size = [r.width(), r.height()];
            if app.panel_world_map.saved_pos != Some(new_pos) || app.panel_world_map.saved_size != Some(new_size) {
                app.panel_world_map.saved_pos = Some(new_pos);
                app.panel_world_map.saved_size = Some(new_size);
                app.save_station_config();
            }
        }
    }

    if !is_open {
        app.panel_world_map.visible = false;
        app.show_world_map_window = false;
        app.save_station_config();
    }
}


pub fn render_world_map_content(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.current_language;
    ui.vertical(|ui| {
        // Obliczenie aktualnej pozycji słońca (UTC)
        let now = chrono::Utc::now();
        let day_of_year = now.format("%j").to_string().parse::<u32>().unwrap_or(263);
        let utc_hour_f64 = now.format("%H").to_string().parse::<f64>().unwrap_or(12.0)
            + now.format("%M").to_string().parse::<f64>().unwrap_or(0.0) / 60.0;

        // Współrzędne mojej stacji
        let my_coords = locator_to_coordinates(&app.my_station.gridsquare)
            .unwrap_or(Coordinates::new(51.1079, 17.0385));

        // Współrzędne stacji DX
        let dx_coords = if app.entry_grid.len() >= 4 {
            locator_to_coordinates(&app.entry_grid).ok()
        } else { app.active_prefix_info.as_ref().map(|info| Coordinates::new(info.latitude, info.longitude)) };

        let my_solar = calculate_solar_position(my_coords, day_of_year, utc_hour_f64);
        
        let day_of_year_f64 = day_of_year as f64;
        let declination = -23.45 * (2.0 * std::f64::consts::PI / 365.0 * (day_of_year_f64 + 10.0)).cos();
        let subsolar_lat = declination;
        let mut subsolar_lon = -(utc_hour_f64 * 15.0 - 180.0);
        while subsolar_lon < -180.0 { subsolar_lon += 360.0; }
        while subsolar_lon > 180.0 { subsolar_lon -= 360.0; }

        // Pasek statusu propagacji i Grayline
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("Grayline:").strong().size(11.0).color(egui::Color32::from_rgb(56, 189, 248)));
            if my_solar.is_grayline {
                ui.colored_label(egui::Color32::from_rgb(34, 197, 94), format!("● {}", tr("map.my_in_grayline", lang)));
            } else if my_solar.elevation > 0.0 {
                ui.colored_label(egui::Color32::from_rgb(250, 204, 21), format!("☀ {}", tr("map.day", lang)));
            } else {
                ui.colored_label(egui::Color32::from_rgb(147, 197, 253), format!("🌙 {}", tr("map.night", lang)));
            }

            if let Some(dx_c) = dx_coords {
                let dx_solar = calculate_solar_position(dx_c, day_of_year, utc_hour_f64);
                ui.separator();
                ui.label(egui::RichText::new(format!("DX: {:.0} km | Azymut: {:.0}°", app.active_distance_km, app.active_bearing_deg)).strong().size(11.0));
                if ui.small_button(format!("🔄 Obróć ({:.0}°)", app.active_bearing_deg))
                    .on_hover_text("Ustawia rotor antenowy na azymut korespondenta przez rotctld")
                    .clicked()
                {
                    app.rotate_antenna_to(app.active_bearing_deg as f32);
                }
                if dx_solar.is_grayline {
                    ui.colored_label(egui::Color32::from_rgb(34, 197, 94), format!("● {}", tr("map.dx_in_grayline", lang)));
                }
            }

            ui.separator();
            ui.checkbox(&mut app.map_show_beam_lobe, format!("📡 {}", tr("map.beam_lobe", lang)));
            ui.checkbox(&mut app.map_show_compass, format!("🧭 {}", tr("map.compass", lang)));
            ui.separator();
            ui.label(egui::RichText::new(format!("{}: {}", tr("map.spots_on_map", lang), app.cluster_spots.len())).size(10.0).color(egui::Color32::from_rgb(148, 163, 184)));
        });

        ui.add_space(2.0);

        // Płótno Canvas Mapy Świata (Equirectangular projection)
        let canvas_size = ui.available_size_before_wrap();
        let width = canvas_size.x.max(380.0);
        let height = (width * 0.50).min(canvas_size.y.max(220.0));

        let id = ui.id().with("world_map_state");
        let mut state = ui.data_mut(|d| d.get_temp::<MapState>(id).unwrap_or_default());

        let (response, painter) = ui.allocate_painter(egui::vec2(width, height), egui::Sense::click_and_drag());
        let rect = response.rect;

        if response.dragged() {
            state.pan += response.drag_delta();
        }
        if response.hovered() {
            let scroll = ui.input(|i| i.raw_scroll_delta.y);
            if scroll != 0.0 {
                state.zoom = (state.zoom + scroll * 0.001).clamp(1.0, 5.0);
            }
        }
        
        ui.data_mut(|d| d.insert_temp(id, state));

        // Tło oceanu
        let ocean_color = egui::Color32::from_rgb(15, 23, 42);
        painter.rect_filled(rect, 4.0, ocean_color);

        // Zoom +/- buttons
        let zoom_rect = egui::Rect::from_min_size(
            rect.left_bottom() + egui::vec2(10.0, -70.0),
            egui::vec2(30.0, 60.0),
        );
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(zoom_rect), |ui| {
            ui.vertical(|ui| {
                if ui.button("+").clicked() {
                    state.zoom = (state.zoom + 0.5).clamp(1.0, 5.0);
                }
                if ui.button("-").clicked() {
                    state.zoom = (state.zoom - 0.5).clamp(1.0, 5.0);
                }
            });
        });

        // Pomocnik projekcji: (lat, lon) -> Pos2
        let project = |lat: f64, lon: f64| -> egui::Pos2 {
            let base_x = ((lon + 180.0) / 360.0) as f32 * rect.width();
            let base_y = ((90.0 - lat) / 180.0) as f32 * rect.height();
            let center_x = rect.width() / 2.0;
            let center_y = rect.height() / 2.0;
            let zoomed_x = (base_x - center_x) * state.zoom + center_x + state.pan.x;
            let zoomed_y = (base_y - center_y) * state.zoom + center_y + state.pan.y;
            egui::pos2(rect.min.x + zoomed_x, rect.min.y + zoomed_y)
        };

        // Siatka geograficzna: Równik, Zwrotniki, Koła podbiegunowe, Południki
        let grid_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 18);
        let axis_color = egui::Color32::from_rgba_unmultiplied(56, 189, 248, 45);

        // Równik (0°) i Południk zerowy (Greenwich 0°)
        painter.line_segment([project(-90.0, 0.0), project(90.0, 0.0)], egui::Stroke::new(1.0_f32, axis_color));
        painter.line_segment([project(0.0, -180.0), project(0.0, 180.0)], egui::Stroke::new(1.0_f32, axis_color));

        // Południki co 60 stopni
        for lon in &[-120.0, -60.0, 60.0, 120.0] {
            painter.line_segment([project(-85.0, *lon), project(85.0, *lon)], egui::Stroke::new(0.8_f32, grid_color));
        }

        // Równoleżniki: Zwrotnik Raka (+23.5°), Zwrotnik Koziorożca (-23.5°), Koła podbiegunowe (±66.5°)
        for lat in &[66.5, 23.5, -23.5, -66.5] {
            painter.line_segment([project(*lat, -180.0), project(*lat, 180.0)], egui::Stroke::new(0.8_f32, grid_color));
        }

        // Subtelne oznaczenia oceanów (Watermarks)
        let ocean_text_color = egui::Color32::from_rgba_unmultiplied(148, 163, 184, 25);
        let font_ocean = egui::FontId::proportional(9.0);
        painter.text(project(0.0, -140.0), egui::Align2::CENTER_CENTER, "PACIFIC OCEAN", font_ocean.clone(), ocean_text_color);
        painter.text(project(25.0, -38.0), egui::Align2::CENTER_CENTER, "NORTH ATLANTIC", font_ocean.clone(), ocean_text_color);
        painter.text(project(-25.0, -20.0), egui::Align2::CENTER_CENTER, "SOUTH ATLANTIC", font_ocean.clone(), ocean_text_color);
        painter.text(project(-15.0, 75.0), egui::Align2::CENTER_CENTER, "INDIAN OCEAN", font_ocean.clone(), ocean_text_color);
        painter.text(project(82.0, 0.0), egui::Align2::CENTER_CENTER, "ARCTIC OCEAN", font_ocean, ocean_text_color);

        // Kolory kontynentów i linii brzegowej
        let land_fill = egui::Color32::from_rgb(33, 44, 66);
        let land_stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(71, 85, 105));

        // Rysowanie realistycznych konturów kontynentów
        let draw_poly = |coords: &[(f64, f64)]| {
            let pts: Vec<egui::Pos2> = coords.iter().map(|(lat, lon)| project(*lat, *lon)).collect();
            if pts.len() >= 3 {
                painter.add(egui::Shape::convex_polygon(pts.clone(), land_fill, egui::Stroke::NONE));
                let mut line_pts = pts;
                if let Some(p0) = line_pts.first() {
                    line_pts.push(*p0);
                }
                painter.add(egui::Shape::line(line_pts, land_stroke));
            }
        };

        draw_poly(CONTINENT_NORTH_AMERICA);
        draw_poly(GREENLAND);
        draw_poly(CONTINENT_SOUTH_AMERICA);
        draw_poly(CONTINENT_AFRICA);
        draw_poly(MADAGASCAR);
        draw_poly(CONTINENT_EUROPE);
        draw_poly(BRITISH_ISLES);
        draw_poly(IRELAND);
        draw_poly(CONTINENT_ASIA);
        draw_poly(JAPAN);
        draw_poly(CONTINENT_AUSTRALIA);
        draw_poly(NEW_ZEALAND);
        draw_poly(CONTINENT_ANTARCTICA);

        // Obliczenie punktów linii terminatora (krzywa zmierzchu)
        let dec_rad = subsolar_lat.to_radians();
        let mut term_pts = Vec::new();
        let step_deg = 4.0;
        let num_steps = (360.0 / step_deg) as i32;

        for i in 0..=num_steps {
            let lon = -180.0 + i as f64 * step_deg;
            let lha = (lon - subsolar_lon).to_radians();
            let term_lat = if dec_rad.abs() > 0.001 {
                (-lha.cos() / dec_rad.tan()).atan().to_degrees()
            } else {
                0.0
            };
            term_pts.push((term_lat, lon));
        }

        // Cieniowanie nocy (półkula zacieniona)
        let night_overlay_color = egui::Color32::from_rgba_unmultiplied(3, 7, 18, 120);
        for i in 0..term_pts.len().saturating_sub(1) {
            let (lat1, lon1) = term_pts[i];
            let (lat2, lon2) = term_pts[i + 1];
            let pt1 = project(lat1, lon1);
            let pt2 = project(lat2, lon2);

            let (y_night1, y_night2) = if subsolar_lat >= 0.0 {
                (project(-90.0, lon1), project(-90.0, lon2))
            } else {
                (project(90.0, lon1), project(90.0, lon2))
            };

            let quad = vec![pt1, pt2, y_night2, y_night1];
            painter.add(egui::Shape::convex_polygon(quad, night_overlay_color, egui::Stroke::NONE));
        }

        // Pas Grayline (złocista poświata zmierzchowa wokół terminatora)
        let grayline_stroke = egui::Stroke::new(3.0_f32, egui::Color32::from_rgba_unmultiplied(251, 191, 36, 170));
        let grayline_outer = egui::Stroke::new(6.0_f32, egui::Color32::from_rgba_unmultiplied(245, 158, 11, 60));

        let screen_term_pts: Vec<egui::Pos2> = term_pts.iter().map(|(lat, lon)| project(*lat, *lon)).collect();
        for i in 0..screen_term_pts.len().saturating_sub(1) {
            let p1 = screen_term_pts[i];
            let p2 = screen_term_pts[i + 1];
            if (p1.x - p2.x).abs() < rect.width() * 0.5 {
                painter.line_segment([p1, p2], grayline_outer);
                painter.line_segment([p1, p2], grayline_stroke);
            }
        }

        // Słońce w zenicie (Subsolar point)
        let sun_pos = project(subsolar_lat, subsolar_lon);
        painter.circle_filled(sun_pos, 7.0, egui::Color32::from_rgb(250, 204, 21));
        painter.circle_stroke(sun_pos, 10.0, egui::Stroke::new(1.5_f32, egui::Color32::from_rgba_unmultiplied(250, 204, 21, 110)));
        painter.text(sun_pos + egui::vec2(0.0, -12.0), egui::Align2::CENTER_CENTER, "☀", egui::FontId::proportional(13.0), egui::Color32::WHITE);

        // Księżyc (punkt anty-słoneczny na mapie)
        let antisolar_lat = -subsolar_lat;
        let antisolar_lon = ((subsolar_lon + 180.0 + 180.0) % 360.0) - 180.0;
        let moon_pos = project(antisolar_lat, antisolar_lon);
        painter.circle_filled(moon_pos, 5.0, egui::Color32::from_rgb(203, 213, 225));
        painter.text(moon_pos + egui::vec2(0.0, -10.0), egui::Align2::CENTER_CENTER, "🌙", egui::FontId::proportional(11.0), egui::Color32::WHITE);

        // Interaktywne spoty DX Cluster nanoszone na żywo na mapę
        let hover_pos = ui.input(|i| i.pointer.hover_pos());
        let clicked = response.clicked();
        let mut tune_target = None;

        for spot in &app.cluster_spots {
            if let Some(info) = app.prefix_matcher.lookup(&spot.dx_call) {
                let spot_pos = project(info.latitude, info.longitude);
                let spot_color = match spot.band.as_str() {
                    "160m" | "80m" => egui::Color32::from_rgb(192, 132, 252),
                    "40m" => egui::Color32::from_rgb(52, 211, 153),
                    "30m" | "20m" => egui::Color32::from_rgb(56, 189, 248),
                    "17m" | "15m" => egui::Color32::from_rgb(250, 204, 21),
                    "12m" | "10m" => egui::Color32::from_rgb(251, 146, 60),
                    _ => egui::Color32::from_rgb(244, 63, 94),
                };

                // Rysowanie trójkąta (Triangle) dla spotów
                let triangle = vec![
                    spot_pos + egui::vec2(0.0, -4.5),
                    spot_pos + egui::vec2(-4.0, 3.5),
                    spot_pos + egui::vec2(4.0, 3.5),
                ];
                painter.add(egui::Shape::convex_polygon(triangle, spot_color, egui::Stroke::new(1.0_f32, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180))));

                if let Some(hp) = hover_pos {
                    if (hp - spot_pos).length() < 7.0 {
                        // Podpowiedź tooltip
                        let tip_box = egui::Rect::from_min_size(spot_pos + egui::vec2(8.0, -16.0), egui::vec2(110.0, 26.0));
                        painter.rect_filled(tip_box, 3.0, egui::Color32::from_rgba_unmultiplied(15, 23, 42, 235));
                        painter.rect_stroke(tip_box, 3.0, egui::Stroke::new(1.0_f32, spot_color));
                        painter.text(
                            spot_pos + egui::vec2(12.0, -3.0),
                            egui::Align2::LEFT_CENTER,
                            format!("{} ({:.1})
{}", spot.dx_call, spot.frequency_khz, spot.band),
                            egui::FontId::proportional(9.0),
                            spot_color,
                        );

                        if clicked {
                            tune_target = Some((spot.dx_call.clone(), spot.frequency_khz, spot.band.clone()));
                        }
                    }
                }
            }
        }

        if let Some((dx_call, freq, band)) = tune_target {
            app.tune_to_spot(&dx_call, freq, &band);
        }

        // QSO Markers
        for qso in &app.recent_qsos {
            if let Some(grid) = &qso.gridsquare {
                if grid.len() >= 4 {
                    if let Ok(c) = locator_to_coordinates(grid) {
                        let qso_pos = project(c.latitude, c.longitude);
                        let is_lotw = qso.lotw_qsl_rcvd == "Y" || qso.lotw_qsl_rcvd == "V";
                        let color = if is_lotw {
                            egui::Color32::from_rgb(34, 197, 94) // Green
                        } else {
                            egui::Color32::from_rgb(59, 130, 246) // Blue
                        };
                        painter.circle_filled(qso_pos, 2.5, color);
                        painter.circle_stroke(qso_pos, 3.5, egui::Stroke::new(1.0_f32, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 150)));
                        
                        if let Some(hp) = hover_pos {
                            if (hp - qso_pos).length() < 5.0 {
                                let tip_box = egui::Rect::from_min_size(qso_pos + egui::vec2(6.0, -12.0), egui::vec2(90.0, 16.0));
                                painter.rect_filled(tip_box, 3.0, egui::Color32::from_rgba_unmultiplied(15, 23, 42, 235));
                                painter.rect_stroke(tip_box, 3.0, egui::Stroke::new(1.0_f32, color));
                                painter.text(
                                    qso_pos + egui::vec2(10.0, -4.0),
                                    egui::Align2::LEFT_CENTER,
                                    format!("QSO: {}", qso.callsign),
                                    egui::FontId::proportional(9.0),
                                    color,
                                );
                            }
                        }
                    }
                }
            }
        }

        // Znacznik stacji własnej (My QTH) i punkt początkowy wiązki
        let my_pos = project(my_coords.latitude, my_coords.longitude);

        // Rysowanie wiązki promieniowania anteny (Beam Lobe) oraz mechanicznego boomu
        if app.map_show_beam_lobe {
            let az = app.rotor_state.azimuth_deg;
            let half_beam = (app.map_beamwidth_deg * 0.5).clamp(10.0, 60.0);
            let lobe_len = 65.0_f32;

            // Rysowanie wachlarza wiązki (Polygon)
            let mut lobe_pts = vec![my_pos];
            let steps = 12;
            for i in 0..=steps {
                let frac = i as f32 / steps as f32;
                let angle_deg = (az - half_beam) + (half_beam * 2.0) * frac;
                let rad = (angle_deg - 90.0).to_radians();
                let pt = my_pos + egui::vec2(rad.cos() * lobe_len, rad.sin() * lobe_len);
                lobe_pts.push(pt);
            }
            let lobe_fill = egui::Color32::from_rgba_unmultiplied(56, 189, 248, 55);
            painter.add(egui::Shape::convex_polygon(
                lobe_pts,
                lobe_fill,
                egui::Stroke::new(1.0_f32, egui::Color32::from_rgba_unmultiplied(56, 189, 248, 140)),
            ));

            // Oś główna wiązki (Promieniowanie główne - szmaragdowy kolor)
            let main_rad = (az - 90.0).to_radians();
            let main_tip = my_pos + egui::vec2(main_rad.cos() * (lobe_len + 15.0), main_rad.sin() * (lobe_len + 15.0));
            painter.line_segment([my_pos, main_tip], egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(52, 211, 153)));

            // Oś mechaniczna (Boom anteny / tył 180° - subtelny szary)
            let boom_rad = (az + 90.0).to_radians();
            let boom_tip = my_pos + egui::vec2(boom_rad.cos() * 32.0, boom_rad.sin() * 32.0);
            painter.line_segment([my_pos, boom_tip], egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(148, 163, 184)));
        }

        // Znacznik stacji własnej (My QTH)
        painter.circle_filled(my_pos, 6.0, egui::Color32::from_rgb(56, 189, 248));
        painter.circle_stroke(my_pos, 9.0, egui::Stroke::new(1.5_f32, egui::Color32::WHITE));
        painter.text(
            my_pos + egui::vec2(10.0, -5.0),
            egui::Align2::LEFT_CENTER,
            format!("{} ({})", app.my_station.callsign, app.my_station.gridsquare),
            egui::FontId::proportional(10.0),
            egui::Color32::from_rgb(56, 189, 248),
        );

        // Znacznik stacji DX oraz linia ortodromy (Great Circle trajectory)
        if let Some(dx_c) = dx_coords {
            let dx_pos = project(dx_c.latitude, dx_c.longitude);
            painter.circle_filled(dx_pos, 6.0, egui::Color32::from_rgb(239, 68, 68));
            painter.circle_stroke(dx_pos, 9.0, egui::Stroke::new(1.5_f32, egui::Color32::WHITE));

            let dx_label = if !app.entry_callsign.is_empty() {
                app.entry_callsign.clone()
            } else {
                "DX".to_string()
            };
            painter.text(
                dx_pos + egui::vec2(10.0, -5.0),
                egui::Align2::LEFT_CENTER,
                dx_label,
                egui::FontId::proportional(10.0),
                egui::Color32::from_rgb(239, 68, 68),
            );

            // Krzywa ortodromy łącząca stację własną ze stacją DX
            let mut prev_pt = my_pos;
            for step in 1..=24 {
                let t = step as f64 / 24.0;
                let int_lat = my_coords.latitude * (1.0 - t) + dx_c.latitude * t;
                let int_lon = my_coords.longitude * (1.0 - t) + dx_c.longitude * t;
                let pt = project(int_lat, int_lon);
                if (prev_pt.x - pt.x).abs() < rect.width() * 0.5 {
                    painter.line_segment([prev_pt, pt], egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(52, 211, 153)));
                }
                prev_pt = pt;
            }
        }

        // Widżet Róży Kompasu Azymutalnego (Azimuthal Compass Rose Overlay)
        if app.map_show_compass {
            let compass_center = rect.right_top() + egui::vec2(-65.0, 65.0);
            let compass_radius = 48.0_f32;

            // Tarcza kompasu
            painter.circle_filled(compass_center, compass_radius, egui::Color32::from_rgba_unmultiplied(15, 23, 42, 220));
            painter.circle_stroke(compass_center, compass_radius, egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(56, 189, 248)));
            painter.circle_stroke(compass_center, compass_radius * 0.65, egui::Stroke::new(0.8_f32, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 25)));

            // Główne punkty kardynalne: N, E, S, W
            let font_card = egui::FontId::proportional(10.0);
            painter.text(compass_center + egui::vec2(0.0, -compass_radius + 10.0), egui::Align2::CENTER_CENTER, "N", font_card.clone(), egui::Color32::from_rgb(239, 68, 68));
            painter.text(compass_center + egui::vec2(compass_radius - 10.0, 0.0), egui::Align2::CENTER_CENTER, "E", font_card.clone(), egui::Color32::from_rgb(148, 163, 184));
            painter.text(compass_center + egui::vec2(0.0, compass_radius - 10.0), egui::Align2::CENTER_CENTER, "S", font_card.clone(), egui::Color32::from_rgb(148, 163, 184));
            painter.text(compass_center + egui::vec2(-compass_radius + 10.0, 0.0), egui::Align2::CENTER_CENTER, "W", font_card, egui::Color32::from_rgb(148, 163, 184));

            // Wskazówka rotatora (Needle)
            let az = app.rotor_state.azimuth_deg;
            let needle_rad = (az - 90.0).to_radians();
            let needle_tip = compass_center + egui::vec2(needle_rad.cos() * (compass_radius - 6.0), needle_rad.sin() * (compass_radius - 6.0));
            painter.line_segment([compass_center, needle_tip], egui::Stroke::new(2.5_f32, egui::Color32::from_rgb(251, 191, 36)));
            painter.circle_filled(compass_center, 4.0, egui::Color32::from_rgb(251, 191, 36));

            // Odczyt cyfrowy w środku tarczy
            painter.text(
                compass_center + egui::vec2(0.0, 16.0),
                egui::Align2::CENTER_CENTER,
                format!("{:.0}°", az),
                egui::FontId::monospace(10.0),
                egui::Color32::from_rgb(251, 191, 36),
            );

            // Interaktywny klik na tarczy kompasu -> bezpośredni obrót rotatora na kliknięty kąt
            if clicked {
                if let Some(pos) = hover_pos {
                    let diff = pos - compass_center;
                    if diff.length() <= compass_radius {
                        // Kąt w radianach względem osi X (prawo)
                        let click_rad = diff.y.atan2(diff.x);
                        // Konwersja na azymut geograficzny (0° = Północ/góra, 90° = Wschód/prawo)
                        let mut click_az = click_rad.to_degrees() + 90.0;
                        if click_az < 0.0 {
                            click_az += 360.0;
                        }
                        click_az = click_az % 360.0;
                        app.rotate_antenna_to(click_az as f32);
                    }
                }
            }
        }
    });
}
