// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::geo::{Coordinates, calculate_solar_position, locator_to_coordinates};
use crate::core::i18n::{Msg, key, tr};
use crate::gui::app::SpLogApp;
use crate::gui::map_data;
use crate::gui::theme::{ColorExt, ThemeExt, undock_button};
use eframe::egui;

// Kontury lądów: Natural Earth 110m Land (Public Domain), patrz `map_data`.

/// Ikony rysowane wektorowo (bez glifów emoji, które bywają `☐` na części systemów).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapIcon {
    Sun,
    Moon,
    Compass,
    Beam,
}

/// Rysuje ikonę o boku `size` wycentrowaną w `center`.
pub fn paint_map_icon(
    painter: &egui::Painter,
    icon: MapIcon,
    center: egui::Pos2,
    size: f32,
    color: egui::Color32,
) {
    let r = size * 0.5;
    let stroke = egui::Stroke::new((size / 12.0).max(1.0), color);
    match icon {
        MapIcon::Sun => {
            painter.circle_filled(center, r * 0.45, color);
            for i in 0..8_u8 {
                let angle = f32::from(i) * std::f32::consts::FRAC_PI_4;
                let dir = egui::vec2(angle.cos(), angle.sin());
                painter.line_segment([center + dir * (r * 0.65), center + dir * r], stroke);
            }
        }
        MapIcon::Moon => {
            let points = crescent_points(center, r);
            let flat: Vec<[f32; 2]> = points.iter().map(|p| [p.x, p.y]).collect();
            let mut mesh = egui::Mesh::default();
            for point in &points {
                mesh.colored_vertex(*point, color);
            }
            for [a, b, c] in map_data::triangulate(&flat) {
                mesh.add_triangle(a as u32, b as u32, c as u32);
            }
            painter.add(egui::Shape::mesh(mesh));
        }
        MapIcon::Compass => {
            painter.circle_stroke(center, r * 0.9, stroke);
            let tip = center + egui::vec2(0.0, -r * 0.75);
            let tail = center + egui::vec2(0.0, r * 0.75);
            let side = egui::vec2(r * 0.25, 0.0);
            painter.add(egui::Shape::convex_polygon(
                vec![tip, center + side, tail, center - side],
                color,
                egui::Stroke::NONE,
            ));
        }
        MapIcon::Beam => {
            let apex = center + egui::vec2(0.0, r * 0.8);
            let mut arc = Vec::new();
            for i in 0..=8_u8 {
                let angle = (-40.0_f32 + 10.0 * f32::from(i)).to_radians();
                arc.push(apex + egui::vec2(angle.sin(), -angle.cos()) * (r * 1.6));
            }
            painter.line_segment([apex, arc[0]], stroke);
            painter.line_segment([apex, arc[arc.len() - 1]], stroke);
            painter.add(egui::Shape::line(arc, stroke));
        }
    }
}

/// Punkty półksiężyca (wielokąt wklęsły: łuk zewnętrzny + łuk wewnętrzny) o promieniu `r`.
fn crescent_points(center: egui::Pos2, r: f32) -> Vec<egui::Pos2> {
    // Okrąg zewnętrzny (0,0; 1) minus okrąg wewnętrzny (0,5; 0,8): przecięcia w ±52,4° / ±82,1°.
    let mut points = Vec::new();
    for i in 0..=20_u8 {
        let angle = (52.4_f32 + (307.6 - 52.4) * f32::from(i) / 20.0).to_radians();
        points.push(center + egui::vec2(angle.cos(), angle.sin()) * r);
    }
    for i in 0..=20_u8 {
        let angle = (277.9_f32 - (277.9 - 82.1) * f32::from(i) / 20.0).to_radians();
        points.push(center + egui::vec2(0.5 + 0.8 * angle.cos(), 0.8 * angle.sin()) * r);
    }
    points
}

/// Etykieta z ikoną wektorową.
fn icon_label(ui: &mut egui::Ui, icon: MapIcon, color: egui::Color32, text: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
        paint_map_icon(ui.painter(), icon, rect.center(), 14.0, color);
        ui.label(egui::RichText::new(text).color(color));
    });
}

/// Checkbox z ikoną wektorową przed etykietą.
fn icon_checkbox(
    ui: &mut egui::Ui,
    icon: MapIcon,
    color: egui::Color32,
    value: &mut bool,
    text: &str,
) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
        paint_map_icon(ui.painter(), icon, rect.center(), 14.0, color);
        ui.checkbox(value, text);
    });
}

/// Azymut wiązki/kompasu: z rotatora, gdy jest aktywny (podłączony, w ruchu lub z wysłaną
/// komendą), w przeciwnym razie kierunek na korespondenta; bez żadnego źródła — `None`
/// (żadnego mylącego „0°” na stacji domowej).
#[must_use]
pub fn beam_azimuth(
    rotor: &crate::cat::rotor::RotorState,
    dx_bearing_deg: Option<f32>,
) -> Option<f32> {
    if rotor.connected || rotor.moving || rotor.azimuth_deg != 0.0 {
        Some(rotor.azimuth_deg)
    } else {
        dx_bearing_deg.filter(|b| b.is_finite() && *b >= 0.0)
    }
}

/// Etykieta stacji domowej: znak (z lokatorem, jeśli jest), a przy pustym znaku — sam lokator
/// lub nic.
#[must_use]
pub fn home_label(callsign: &str, gridsquare: &str) -> Option<String> {
    match (callsign.trim(), gridsquare.trim()) {
        ("", "") => None,
        ("", grid) => Some(grid.to_string()),
        (call, "") => Some(call.to_string()),
        (call, grid) => Some(format!("{call} ({grid})")),
    }
}

#[derive(Clone, Copy, Debug)]
struct MapState {
    zoom: f32,
    pan: egui::Vec2,
}
impl Default for MapState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: egui::Vec2::ZERO,
        }
    }
}

pub fn render_world_map_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.views.panel_world_map.visible {
        return;
    }

    let lang = app.gui.appearance.language;

    // Obsługa Multi-Viewport (niezależne okno OS)
    if app.views.panel_world_map.floating {
        let mut still_open = true;
        let mut dock_back = false;
        let ((), captured_geo) = app.show_floating_viewport(
            ctx,
            egui::ViewportId::from_hash_of("world_map_viewport"),
            format!("🗺 {} - SPLogbook", tr(key::MAP_WORLD_TITLE, lang)),
            [900.0, 560.0],
            [460.0, 300.0],
            app.views.panel_world_map.saved_pos,
            app.views.panel_world_map.saved_size,
            |app, ui| {
                egui::Panel::top("world_map_vp_bar").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let palette = ui.palette();
                        if crate::gui::theme::dock_button(
                            ui,
                            &palette,
                            tr(key::WINDOW_DOCK, lang),
                            tr(key::WINDOW_DOCK_TOOLTIP, lang),
                        )
                        .clicked()
                        {
                            dock_back = true;
                        }
                    });
                });
                egui::CentralPanel::default().show(ui, |ui| {
                    render_world_map_content(app, ui);
                });
                if ui.ctx().input(|i| i.viewport().close_requested()) {
                    still_open = false;
                }
            },
        );

        if let Some((pos, size)) = captured_geo
            && (app.views.panel_world_map.saved_pos != Some(pos)
                || app.views.panel_world_map.saved_size != Some(size))
        {
            app.views.panel_world_map.saved_pos = Some(pos);
            app.views.panel_world_map.saved_size = Some(size);
            app.save_station_config();
        }
        if dock_back {
            app.dock_tile("world_map");
            app.save_station_config();
        }
        if !still_open {
            app.views.panel_world_map.visible = false;
            app.views.panel_world_map.floating = false;
            app.save_station_config();
        }
        return;
    }

    let mut is_open = app.views.panel_world_map.visible;
    let screen = ctx.content_rect();
    let right_w = 360.0_f32.min((screen.width() - 500.0).max(200.0));
    let mid_w = (screen.width() - 488.0 - right_w - 20.0).max(380.0);

    let default_pos = [screen.min.x + 488.0, screen.min.y + 8.0];
    let default_size = [mid_w, (screen.height() - 20.0).max(450.0)];

    let mut win = egui::Window::new(
        egui::RichText::new(format!("   🗺 {}", tr(key::MAP_WORLD_TITLE, lang)))
            .size(12.0)
            .strong(),
    )
    .open(&mut is_open)
    .min_size([400.0, 300.0])
    .resizable(true)
    .collapsible(true)
    .constrain_to(screen);

    if app.gui.workspace.reset_layout_requested {
        win = win.current_pos(default_pos).default_size(default_size);
    } else if let Some(pos) = app.views.panel_world_map.saved_pos {
        let raw_sz = app.views.panel_world_map.saved_size.unwrap_or(default_size);
        let max_w = (screen.width() - 24.0).max(420.0);
        let max_h = (screen.height() - 24.0).max(320.0);
        let sz = [raw_sz[0].clamp(400.0, max_w), raw_sz[1].clamp(300.0, max_h)];
        win = win
            .current_pos(pos)
            .default_size(sz)
            .max_size([max_w, max_h]);
    } else {
        let max_w = (screen.width() - 24.0).max(420.0);
        let max_h = (screen.height() - 24.0).max(320.0);
        win = win
            .default_pos(default_pos)
            .default_size(default_size)
            .max_size([max_w, max_h]);
    }

    let win_res = win.show(ctx, |ui| {
        render_world_map_content(app, ui);
    });

    if let Some(ref res) = win_res {
        crate::gui::render_titlebar_popout_button_if(
            ctx,
            "world_map_popout_btn",
            res.response.layer_id,
            res.response.rect,
            &mut app.views.panel_world_map.floating,
            true,
            app.gui.appearance.language,
        );
        if app.views.panel_world_map.floating {
            app.save_station_config();
        }
        if res.response.dragged() || res.response.drag_stopped() {
            let rect = res.response.rect;
            let new_pos = [rect.min.x, rect.min.y];
            let new_size = [rect.width(), rect.height()];
            app.views.panel_world_map.saved_pos = Some(new_pos);
            app.views.panel_world_map.saved_size = Some(new_size);
            if res.response.drag_stopped() {
                app.save_station_config();
            }
        }
    }

    if !is_open {
        app.views.panel_world_map.visible = false;
        app.world_map.show_window = false;
        app.save_station_config();
    }
}

pub fn render_world_map_content(app: &mut SpLogApp, ui: &mut egui::Ui) {
    let lang = app.gui.appearance.language;
    let palette = ui.palette();
    let map = palette.map;
    ui.vertical(|ui| {
        // Obliczenie aktualnej pozycji słońca (UTC)
        let now = jiff::tz::TimeZone::UTC.to_datetime(jiff::Timestamp::now());
        let day_of_year = now.day_of_year() as u32;
        let utc_hour_f64 = now.hour() as f64 + now.minute() as f64 / 60.0;

        // Współrzędne mojej stacji
        let my_coords = locator_to_coordinates(&app.config.station.gridsquare)
            .unwrap_or(Coordinates::new(51.1079, 17.0385));

        // Współrzędne stacji DX
        let dx_coords = if app.entry.grid.len() >= 4 {
            locator_to_coordinates(&app.entry.grid).ok()
        } else {
            app.entry
                .active_prefix_info
                .as_ref()
                .map(|info| Coordinates::new(info.latitude, info.longitude))
        };

        let my_solar = calculate_solar_position(my_coords, day_of_year, utc_hour_f64);
        let beam_az = beam_azimuth(
            &app.cat.rotor_state,
            dx_coords.map(|_| app.entry.active_bearing_deg as f32),
        );

        let day_of_year_f64 = day_of_year as f64;
        let declination =
            -23.45 * (2.0 * std::f64::consts::PI / 365.0 * (day_of_year_f64 + 10.0)).cos();
        let subsolar_lat = declination;
        let mut subsolar_lon = -(utc_hour_f64 * 15.0 - 180.0);
        while subsolar_lon < -180.0 {
            subsolar_lon += 360.0;
        }
        while subsolar_lon > 180.0 {
            subsolar_lon -= 360.0;
        }

        // Pasek narzędzi mapy: Grayline, azymut DX, warstwy; po prawej przycisk odpinania
        // (wkomponowany w pasek, bez osobnego wiersza).
        let docked = !app.views.panel_world_map.floating;
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new(format!("{}:", tr(key::GEO_GRAYLINE, lang)))
                    .strong()
                    .size(11.0)
                    .color(palette.accent),
            );
            if my_solar.is_grayline {
                ui.colored_label(
                    palette.success,
                    format!("● {}", tr(key::MAP_MY_IN_GRAYLINE, lang)),
                );
            } else if my_solar.elevation > 0.0 {
                icon_label(
                    ui,
                    MapIcon::Sun,
                    palette.text_primary,
                    tr(key::MAP_DAY, lang),
                );
            } else {
                icon_label(ui, MapIcon::Moon, palette.accent, tr(key::MAP_NIGHT, lang));
            }

            if let Some(dx_c) = dx_coords {
                let dx_solar = calculate_solar_position(dx_c, day_of_year, utc_hour_f64);
                ui.separator();
                ui.label(
                    egui::RichText::new(
                        Msg::new(key::MAP_DX_AZIMUTH)
                            .arg("dist", app.active_distance_display())
                            .arg("bearing", format!("{:.0}", app.entry.active_bearing_deg))
                            .text(lang),
                    )
                    .strong()
                    .size(11.0),
                );
                if ui
                    .small_button(
                        Msg::new(key::QSO_ROTATE_BEARING)
                            .arg("bearing", format!("{:.0}", app.entry.active_bearing_deg))
                            .text(lang),
                    )
                    .on_hover_text(tr(key::MAP_ROTOR_TOOLTIP, lang))
                    .clicked()
                {
                    app.rotate_antenna_to(app.entry.active_bearing_deg as f32);
                }
                if dx_solar.is_grayline {
                    ui.colored_label(
                        palette.success,
                        format!("● {}", tr(key::MAP_DX_IN_GRAYLINE, lang)),
                    );
                }
            }

            ui.separator();
            icon_checkbox(
                ui,
                MapIcon::Beam,
                palette.text_muted,
                &mut app.world_map.show_beam_lobe,
                tr(key::MAP_BEAM_LOBE, lang),
            );
            icon_checkbox(
                ui,
                MapIcon::Compass,
                palette.text_muted,
                &mut app.world_map.show_compass,
                tr(key::MAP_COMPASS, lang),
            );
            ui.checkbox(
                &mut app.world_map.show_grid,
                tr(key::MAP_LOCATOR_GRID, lang),
            );
            ui.separator();
            ui.label(
                egui::RichText::new(format!(
                    "{}: {}",
                    tr(key::MAP_SPOTS_ON_MAP, lang),
                    app.cluster.spots.len()
                ))
                .size(10.0)
                .color(palette.text_muted),
            );
            if docked {
                let free = ui.available_width() - 28.0;
                if free > 0.0 {
                    ui.add_space(free);
                }
                if undock_button(ui, &palette, tr(key::WINDOW_POPOUT_TOOLTIP, lang)).clicked() {
                    app.popout_tile("world_map");
                }
            }
        });

        ui.add_space(2.0);

        // Płótno Canvas Mapy Świata (Equirectangular projection)
        let avail = ui.available_size();
        let width = avail.x.clamp(240.0, 1600.0);
        let height = (width * 0.50)
            .min((avail.y - 4.0).max(180.0))
            .clamp(180.0, 900.0);

        let id = ui.id().with("world_map_state");
        let mut state = ui.data_mut(|d| d.get_temp::<MapState>(id).unwrap_or_default());

        let (response, raw_painter) =
            ui.allocate_painter(egui::vec2(width, height), egui::Sense::click_and_drag());
        let rect = response.rect;
        let painter = raw_painter.with_clip_rect(rect);

        if response.dragged() {
            state.pan += response.drag_delta();
        }
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                state.zoom = (state.zoom + scroll * 0.001).clamp(1.0, 5.0);
            }
        }

        // Tło oceanu
        let ocean_color = map.ocean;
        painter.rect_filled(rect, 4.0, ocean_color);

        // Zoom +/- buttons
        let zoom_rect = egui::Rect::from_min_size(
            rect.left_bottom() + egui::vec2(10.0, -70.0),
            egui::vec2(30.0, 60.0),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(zoom_rect), |ui| {
            ui.vertical(|ui| {
                if ui.button("+").clicked() {
                    state.zoom = (state.zoom + 0.5).clamp(1.0, 5.0);
                }
                if ui.button("-").clicked() {
                    state.zoom = (state.zoom - 0.5).clamp(1.0, 5.0);
                }
            });
        });

        ui.data_mut(|d| d.insert_temp(id, state));

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
        let grid_color = map.graticule;
        let axis_color = map.axis;

        // Równik (0°) i Południk zerowy (Greenwich 0°)
        painter.line_segment(
            [project(-90.0, 0.0), project(90.0, 0.0)],
            egui::Stroke::new(1.0_f32, axis_color),
        );
        painter.line_segment(
            [project(0.0, -180.0), project(0.0, 180.0)],
            egui::Stroke::new(1.0_f32, axis_color),
        );

        // Południki co 60 stopni
        for lon in &[-120.0, -60.0, 60.0, 120.0] {
            painter.line_segment(
                [project(-85.0, *lon), project(85.0, *lon)],
                egui::Stroke::new(0.8_f32, grid_color),
            );
        }

        // Równoleżniki: Zwrotnik Raka (+23.5°), Zwrotnik Koziorożca (-23.5°), Koła podbiegunowe (±66.5°)
        for lat in &[66.5, 23.5, -23.5, -66.5] {
            painter.line_segment(
                [project(*lat, -180.0), project(*lat, 180.0)],
                egui::Stroke::new(0.8_f32, grid_color),
            );
        }

        // Subtelne oznaczenia oceanów (Watermarks)
        let ocean_text_color = map.ocean_text;
        let font_ocean = egui::FontId::proportional(9.0);
        painter.text(
            project(0.0, -140.0),
            egui::Align2::CENTER_CENTER,
            "PACIFIC OCEAN",
            font_ocean.clone(),
            ocean_text_color,
        );
        painter.text(
            project(25.0, -38.0),
            egui::Align2::CENTER_CENTER,
            "NORTH ATLANTIC",
            font_ocean.clone(),
            ocean_text_color,
        );
        painter.text(
            project(-25.0, -20.0),
            egui::Align2::CENTER_CENTER,
            "SOUTH ATLANTIC",
            font_ocean.clone(),
            ocean_text_color,
        );
        painter.text(
            project(-15.0, 75.0),
            egui::Align2::CENTER_CENTER,
            "INDIAN OCEAN",
            font_ocean.clone(),
            ocean_text_color,
        );
        painter.text(
            project(82.0, 0.0),
            egui::Align2::CENTER_CENTER,
            "ARCTIC OCEAN",
            font_ocean,
            ocean_text_color,
        );

        // Lądy: Natural Earth 110m Land (Public Domain) — jeden `Mesh` z poligonów
        // triangulowanych raz (`OnceLock`) + cienki obrys linii brzegowej.
        let geometry = map_data::land();
        let screen_vertices: Vec<egui::Pos2> = geometry
            .vertices
            .iter()
            .map(|[lon, lat]| project(f64::from(*lat), f64::from(*lon)))
            .collect();
        let build_mesh = |indices: &[u32], color: egui::Color32| {
            let mut mesh = egui::Mesh::default();
            mesh.vertices = screen_vertices
                .iter()
                .map(|&pos| egui::epaint::Vertex {
                    pos,
                    uv: egui::epaint::WHITE_UV,
                    color,
                })
                .collect();
            mesh.indices = indices.to_vec();
            mesh
        };
        painter.add(egui::Shape::mesh(build_mesh(&geometry.land, map.land)));
        // Dziury (np. Morze Kaspijskie) w kolorze oceanu.
        painter.add(egui::Shape::mesh(build_mesh(&geometry.holes, map.ocean)));
        let coast_stroke = egui::Stroke::new(1.0_f32, map.coastline);
        for &(start, len, _hole) in &geometry.rings {
            let mut outline: Vec<egui::Pos2> = screen_vertices[start..start + len].to_vec();
            if let Some(first) = outline.first().copied() {
                outline.push(first);
            }
            painter.add(egui::Shape::line(outline, coast_stroke));
        }

        // Siatka pól Maidenhead (AA..RR) z etykietami, gdy pole jest dość duże na ekranie.
        if app.world_map.show_grid {
            let grid_stroke = egui::Stroke::new(0.8_f32, map.locator_grid);
            for i in 0..=18 {
                let lon = -180.0 + f64::from(i) * 20.0;
                painter.line_segment([project(90.0, lon), project(-90.0, lon)], grid_stroke);
            }
            for j in 0..=18 {
                let lat = -90.0 + f64::from(j) * 10.0;
                painter.line_segment([project(lat, -180.0), project(lat, 180.0)], grid_stroke);
            }
            let cell_px = rect.width() / 18.0 * state.zoom;
            if cell_px >= 26.0 {
                for lon_index in 0..18_usize {
                    for lat_index in 0..18_usize {
                        let lon = -180.0 + (lon_index as f64 + 0.5) * 20.0;
                        let lat = -90.0 + (lat_index as f64 + 0.5) * 10.0;
                        painter.text(
                            project(lat, lon),
                            egui::Align2::CENTER_CENTER,
                            map_data::maidenhead_field(lon_index, lat_index),
                            egui::FontId::proportional(10.0),
                            map.locator_grid,
                        );
                    }
                }
            }
        }

        // Cień nocy i zmierzch: jeden `Mesh` ze współdzielonymi wierzchołkami (kolumny co 4°,
        // wiersze: krzywe zenitalne 84°/90°/96°/108° i biegun nocy). Brak osobnych
        // wielokątów półprzezroczystych = brak szwów antyaliasingu (pionowych prążków).
        let step_deg = 4.0;
        let num_steps = (360.0 / step_deg) as usize;
        let night_columns: Vec<[f64; 5]> = (0..=num_steps)
            .map(|i| {
                map_data::night_column(-180.0 + i as f64 * step_deg, subsolar_lat, subsolar_lon)
            })
            .collect();
        let mut night_mesh = egui::Mesh::default();
        for (i, column) in night_columns.iter().enumerate() {
            let lon = -180.0 + i as f64 * step_deg;
            for (row, lat) in column.iter().enumerate() {
                night_mesh.colored_vertex(
                    project(*lat, lon),
                    map.night_overlay
                        .gamma_multiply(map_data::NIGHT_ALPHAS[row]),
                );
            }
        }
        for i in 0..night_columns.len().saturating_sub(1) {
            for row in 0..4_usize {
                let a = (i * 5 + row) as u32;
                let b = a + 1;
                let c = ((i + 1) * 5 + row) as u32;
                let d = c + 1;
                night_mesh.add_triangle(a, b, c);
                night_mesh.add_triangle(b, d, c);
            }
        }
        painter.add(egui::Shape::mesh(night_mesh));

        // Pas Grayline (złocista poświata zmierzchowa wokół terminatora = krzywa zenitalna 90°)
        let grayline_stroke = egui::Stroke::new(3.0_f32, map.terminator);
        let grayline_outer = egui::Stroke::new(6.0_f32, map.terminator.alpha(60));
        let screen_term_pts: Vec<egui::Pos2> = night_columns
            .iter()
            .enumerate()
            .map(|(i, column)| project(column[1], -180.0 + i as f64 * step_deg))
            .collect();
        for pair in screen_term_pts.windows(2) {
            let (p1, p2) = (pair[0], pair[1]);
            if (p1.x - p2.x).abs() < rect.width() * 0.5 {
                painter.line_segment([p1, p2], grayline_outer);
                painter.line_segment([p1, p2], grayline_stroke);
            }
        }

        // Słońce w zenicie (Subsolar point)
        let sun_pos = project(subsolar_lat, subsolar_lon);
        painter.circle_stroke(sun_pos, 11.0, egui::Stroke::new(1.5_f32, map.sun_halo));
        paint_map_icon(&painter, MapIcon::Sun, sun_pos, 18.0, map.sun);

        // Księżyc (punkt anty-słoneczny na mapie)
        let antisolar_lat = -subsolar_lat;
        let antisolar_lon = ((subsolar_lon + 180.0 + 180.0) % 360.0) - 180.0;
        let moon_pos = project(antisolar_lat, antisolar_lon);
        paint_map_icon(&painter, MapIcon::Moon, moon_pos, 14.0, map.moon);

        // Interaktywne spoty DX Cluster nanoszone na żywo na mapę
        let hover_pos = ui.input(|i| i.pointer.hover_pos());
        let clicked = response.clicked();
        let mut tune_target = None;

        for spot in &app.cluster.spots {
            if let Some(info) = app.services.prefix_matcher.lookup(&spot.dx_call) {
                let spot_pos = project(info.latitude, info.longitude);
                let spot_color = palette.band_color(&spot.band);

                // Rysowanie trójkąta (Triangle) dla spotów
                let triangle = vec![
                    spot_pos + egui::vec2(0.0, -4.5),
                    spot_pos + egui::vec2(-4.0, 3.5),
                    spot_pos + egui::vec2(4.0, 3.5),
                ];
                painter.add(egui::Shape::convex_polygon(
                    triangle,
                    spot_color,
                    egui::Stroke::new(1.0_f32, map.ocean.alpha(180)),
                ));

                if let Some(hp) = hover_pos
                    && (hp - spot_pos).length() < 7.0
                {
                    // Podpowiedź tooltip
                    let tip_box = egui::Rect::from_min_size(
                        spot_pos + egui::vec2(8.0, -16.0),
                        egui::vec2(110.0, 26.0),
                    );
                    painter.rect_filled(tip_box, 3.0, map.ocean.alpha(235));
                    painter.rect_stroke(
                        tip_box,
                        3.0,
                        egui::Stroke::new(1.0_f32, spot_color),
                        egui::StrokeKind::Inside,
                    );
                    painter.text(
                        spot_pos + egui::vec2(12.0, -3.0),
                        egui::Align2::LEFT_CENTER,
                        format!(
                            "{} ({:.1})
{}",
                            spot.dx_call, spot.frequency_khz, spot.band
                        ),
                        egui::FontId::proportional(9.0),
                        spot_color,
                    );

                    if clicked {
                        tune_target =
                            Some((spot.dx_call.clone(), spot.frequency_khz, spot.band.clone()));
                    }
                }
            }
        }

        if let Some((dx_call, freq, band)) = tune_target {
            app.tune_to_spot(&dx_call, freq, &band);
        }

        // QSO Markers
        for qso in &app.log_view.recent_qsos {
            if let Some(grid) = &qso.gridsquare
                && grid.len() >= 4
                && let Ok(c) = locator_to_coordinates(grid)
            {
                let qso_pos = project(c.latitude, c.longitude);
                let is_lotw = qso.lotw_qsl_rcvd == "Y" || qso.lotw_qsl_rcvd == "V";
                let color = if is_lotw {
                    map.qso_marker_confirmed
                } else {
                    map.qso_marker
                };
                painter.circle_filled(qso_pos, 2.5, color);
                painter.circle_stroke(
                    qso_pos,
                    3.5,
                    egui::Stroke::new(1.0_f32, map.ocean.alpha(150)),
                );

                if let Some(hp) = hover_pos
                    && (hp - qso_pos).length() < 5.0
                {
                    let tip_box = egui::Rect::from_min_size(
                        qso_pos + egui::vec2(6.0, -12.0),
                        egui::vec2(90.0, 16.0),
                    );
                    painter.rect_filled(tip_box, 3.0, map.ocean.alpha(235));
                    painter.rect_stroke(
                        tip_box,
                        3.0,
                        egui::Stroke::new(1.0_f32, color),
                        egui::StrokeKind::Inside,
                    );
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

        // Znacznik stacji własnej (My QTH) i punkt początkowy wiązki
        let my_pos = project(my_coords.latitude, my_coords.longitude);

        // Rysowanie wiązki promieniowania anteny (Beam Lobe) oraz mechanicznego boomu
        if app.world_map.show_beam_lobe
            && let Some(az) = beam_az
        {
            let half_beam = (app.world_map.beamwidth_deg * 0.5).clamp(10.0, 60.0);
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
            let lobe_fill = map.beam_fill;
            painter.add(egui::Shape::convex_polygon(
                lobe_pts,
                lobe_fill,
                egui::Stroke::new(1.0_f32, map.beam_stroke),
            ));

            // Oś główna wiązki (Promieniowanie główne - szmaragdowy kolor)
            let main_rad = (az - 90.0).to_radians();
            let main_tip = my_pos
                + egui::vec2(
                    main_rad.cos() * (lobe_len + 15.0),
                    main_rad.sin() * (lobe_len + 15.0),
                );
            painter.line_segment(
                [my_pos, main_tip],
                egui::Stroke::new(2.0_f32, palette.success),
            );

            // Oś mechaniczna (Boom anteny / tył 180° - subtelny szary)
            let boom_rad = (az + 90.0).to_radians();
            let boom_tip = my_pos + egui::vec2(boom_rad.cos() * 32.0, boom_rad.sin() * 32.0);
            painter.line_segment(
                [my_pos, boom_tip],
                egui::Stroke::new(1.5_f32, palette.text_muted),
            );
        }

        // Znacznik stacji własnej (My QTH)
        painter.circle_filled(my_pos, 6.0, palette.accent);
        painter.circle_stroke(
            my_pos,
            9.0,
            egui::Stroke::new(1.5_f32, palette.text_primary),
        );
        if let Some(label) =
            home_label(&app.config.station.callsign, &app.config.station.gridsquare)
        {
            painter.text(
                my_pos + egui::vec2(10.0, -5.0),
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::proportional(10.0),
                palette.accent,
            );
        }

        // Znacznik stacji DX oraz linia ortodromy (Great Circle trajectory - sferyczna interpolacja SLERP)
        if let Some(dx_c) = dx_coords {
            let dx_pos = project(dx_c.latitude, dx_c.longitude);
            painter.circle_filled(dx_pos, 6.0, palette.danger);
            painter.circle_stroke(
                dx_pos,
                9.0,
                egui::Stroke::new(1.5_f32, palette.text_primary),
            );

            let dx_label = if app.entry.callsign.is_empty() {
                "DX".to_string()
            } else {
                app.entry.callsign.clone()
            };
            painter.text(
                dx_pos + egui::vec2(10.0, -5.0),
                egui::Align2::LEFT_CENTER,
                dx_label,
                egui::FontId::proportional(10.0),
                palette.danger,
            );

            // Wielkie Koło (najkrótsza ścieżka) ze stacji własnej do DX, dzielone na
            // antypołudniku ±180° (łuk dochodzi do krawędzi mapy i wraca po drugiej stronie).
            for segment in map_data::great_circle_segments(
                (my_coords.latitude, my_coords.longitude),
                (dx_c.latitude, dx_c.longitude),
                72,
            ) {
                let pts: Vec<egui::Pos2> = segment
                    .iter()
                    .map(|(lat, lon)| project(*lat, *lon))
                    .collect();
                painter.add(egui::Shape::line(
                    pts,
                    egui::Stroke::new(1.5_f32, palette.success),
                ));
            }
        }

        // Widżet Róży Kompasu Azymutalnego (Azimuthal Compass Rose Overlay)
        if app.world_map.show_compass {
            let compass_center = rect.right_top() + egui::vec2(-65.0, 65.0);
            let compass_radius = 48.0_f32;

            // Tarcza kompasu
            painter.circle_filled(compass_center, compass_radius, map.ocean.alpha(220));
            painter.circle_stroke(
                compass_center,
                compass_radius,
                egui::Stroke::new(1.5_f32, map.compass_ring),
            );
            painter.circle_stroke(
                compass_center,
                compass_radius * 0.65,
                egui::Stroke::new(0.8_f32, map.graticule),
            );

            // Główne punkty kardynalne: N, E, S, W
            let font_card = egui::FontId::proportional(10.0);
            painter.text(
                compass_center + egui::vec2(0.0, -compass_radius + 10.0),
                egui::Align2::CENTER_CENTER,
                "N",
                font_card.clone(),
                palette.danger,
            );
            painter.text(
                compass_center + egui::vec2(compass_radius - 10.0, 0.0),
                egui::Align2::CENTER_CENTER,
                "E",
                font_card.clone(),
                map.compass_text,
            );
            painter.text(
                compass_center + egui::vec2(0.0, compass_radius - 10.0),
                egui::Align2::CENTER_CENTER,
                "S",
                font_card.clone(),
                map.compass_text,
            );
            painter.text(
                compass_center + egui::vec2(-compass_radius + 10.0, 0.0),
                egui::Align2::CENTER_CENTER,
                "W",
                font_card,
                map.compass_text,
            );

            // Wskazówka i odczyt cyfrowy tylko gdy znamy azymut (rotator lub kierunek na DX)
            if let Some(az) = beam_az {
                let needle_rad = (az - 90.0).to_radians();
                let needle_tip = compass_center
                    + egui::vec2(
                        needle_rad.cos() * (compass_radius - 6.0),
                        needle_rad.sin() * (compass_radius - 6.0),
                    );
                painter.line_segment(
                    [compass_center, needle_tip],
                    egui::Stroke::new(2.5_f32, palette.accent),
                );
                painter.circle_filled(compass_center, 4.0, palette.accent);
                painter.text(
                    compass_center + egui::vec2(0.0, 16.0),
                    egui::Align2::CENTER_CENTER,
                    format!("{az:.0}°"),
                    egui::FontId::monospace(10.0),
                    palette.accent,
                );
            }

            // Interaktywny klik na tarczy kompasu -> bezpośredni obrót rotatora na kliknięty kąt
            if clicked && let Some(pos) = hover_pos {
                let diff = pos - compass_center;
                if diff.length() <= compass_radius {
                    // Kąt w radianach względem osi X (prawo)
                    let click_rad = diff.y.atan2(diff.x);
                    // Konwersja na azymut geograficzny (0° = Północ/góra, 90° = Wschód/prawo)
                    let mut click_az = click_rad.to_degrees() + 90.0;
                    if click_az < 0.0 {
                        click_az += 360.0;
                    }
                    click_az %= 360.0;
                    app.rotate_antenna_to(click_az as f32);
                }
            }
        }
    });
}
