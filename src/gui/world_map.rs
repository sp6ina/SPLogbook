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
///
/// Bez zagnieżdżonego `ui.horizontal`: w pasku z zawijaniem zagnieżdżony układ nie zawija
/// się i rozpychałby panel poza szerokość kafelka.
fn icon_label(ui: &mut egui::Ui, icon: MapIcon, color: egui::Color32, text: &str) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
    paint_map_icon(ui.painter(), icon, rect.center(), 14.0, color);
    ui.label(egui::RichText::new(text).color(color));
}

/// Checkbox z ikoną wektorową przed etykietą.
fn icon_checkbox(
    ui: &mut egui::Ui,
    icon: MapIcon,
    color: egui::Color32,
    value: &mut bool,
    text: &str,
) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
    paint_map_icon(ui.painter(), icon, rect.center(), 14.0, color);
    ui.checkbox(value, text);
}

/// Appends a filled octagon (a dot at marker size) to `mesh` as a triangle fan.
fn push_dot(mesh: &mut egui::epaint::Mesh, center: egui::Pos2, radius: f32, color: egui::Color32) {
    const SIDES: u32 = 8;
    let Ok(base) = u32::try_from(mesh.vertices.len()) else {
        return;
    };
    mesh.colored_vertex(center, color);
    for i in 0..SIDES {
        let angle = std::f32::consts::TAU * i as f32 / SIDES as f32;
        mesh.colored_vertex(
            center + radius * egui::vec2(angle.cos(), angle.sin()),
            color,
        );
    }
    for i in 0..SIDES {
        mesh.add_triangle(base, base + 1 + i, base + 1 + (i + 1) % SIDES);
    }
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

/// Stan widoku mapy. `pan` jest znormalizowany względem rozmiaru prostokąta rzutu
/// (1.0 = szerokość/wysokość mapy), więc zmiana rozmiaru kafelka nie wyrzuca mapy z kadru.
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

const MIN_ZOOM: f32 = 1.0;
const MAX_ZOOM: f32 = 5.0;
/// Najmniejszy bok płótna, przy którym rysowany jest kompas (poniżej zasłaniałby mapę).
const COMPASS_MIN_CANVAS: f32 = 150.0;
/// Najmniejszy bok płótna, przy którym pokazywane są przyciski przybliżania.
const ZOOM_BUTTONS_MIN_CANVAS: f32 = 110.0;

/// Największy prostokąt o proporcji `2:1` (360° x 180°) mieszczący się w `canvas`,
/// wyśrodkowany (letterbox). Pozostała część płótna jest wypełniana kolorem oceanu.
fn fit_map_rect(canvas: egui::Rect) -> egui::Rect {
    let width = canvas.width().max(0.0);
    let height = canvas.height().max(0.0);
    let (map_w, map_h) = if width >= height * 2.0 {
        (height * 2.0, height)
    } else {
        (width, width / 2.0)
    };
    let inset = egui::vec2((width - map_w) / 2.0, (height - map_h) / 2.0);
    egui::Rect::from_min_max(canvas.min + inset, canvas.max - inset)
}

/// Ogranicza znormalizowane przesunięcie tak, aby przybliżona mapa zawsze pokrywała
/// płótno (brak pustych pasów); przy `zoom <= 1` mapa wraca idealnie na środek.
fn clamp_pan(pan: egui::Vec2, zoom: f32, canvas: egui::Rect, map: egui::Rect) -> egui::Vec2 {
    if zoom.is_nan() || zoom <= MIN_ZOOM || !pan.x.is_finite() || !pan.y.is_finite() {
        return egui::Vec2::ZERO;
    }
    let axis = |value: f32, map_size: f32, canvas_size: f32| -> f32 {
        if map_size <= 0.0 {
            return 0.0;
        }
        let limit = ((map_size * zoom - canvas_size) / 2.0).max(0.0);
        (value * map_size).clamp(-limit, limit) / map_size
    };
    egui::vec2(
        axis(pan.x, map.width(), canvas.width()),
        axis(pan.y, map.height(), canvas.height()),
    )
}

/// Promień tarczy kompasu proporcjonalny do mniejszego boku płótna; `None`, gdy płótno
/// jest tak małe, że kompas zasłaniałby mapę.
fn compass_radius(canvas: egui::Rect) -> Option<f32> {
    let side = canvas.width().min(canvas.height());
    (side >= COMPASS_MIN_CANVAS).then(|| (side * 0.14).clamp(26.0, 48.0))
}

/// Środek tarczy kompasu w prawym górnym rogu płótna (zawsze w całości wewnątrz).
fn compass_center(canvas: egui::Rect, radius: f32) -> egui::Pos2 {
    canvas.right_top() + egui::vec2(-(radius + 8.0), radius + 8.0)
}

/// Współczynnik skali nakładek (wiązka, bum) względem mniejszego boku płótna.
fn overlay_scale(canvas: egui::Rect) -> f32 {
    (canvas.width().min(canvas.height()) / 400.0).clamp(0.5, 1.0)
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

        // Pasek narzędzi mapy: Grayline, azymut DX, warstwy (z zawijaniem, więc nie wymusza
        // szerokości większej niż kafelek); przycisk odpinania jest przypięty w prawym górnym
        // rogu paska w zarezerwowanym pasie (bez osobnego wiersza).
        let docked = !app.views.panel_world_map.floating;
        let toolbar_area = ui.available_rect_before_wrap();
        let undock_reserve = if docked { 28.0 } else { 0.0 };
        let toolbar_rect = egui::Rect::from_min_max(
            toolbar_area.min,
            egui::pos2(
                (toolbar_area.max.x - undock_reserve).max(toolbar_area.min.x + 1.0),
                toolbar_area.max.y,
            ),
        );
        if docked {
            let button_rect = egui::Rect::from_min_size(
                egui::pos2(toolbar_area.max.x - 26.0, toolbar_area.min.y),
                egui::vec2(24.0, 24.0),
            );
            let clicked = ui
                .scope_builder(egui::UiBuilder::new().max_rect(button_rect), |ui| {
                    undock_button(ui, &palette, tr(key::WINDOW_POPOUT_TOOLTIP, lang)).clicked()
                })
                .inner;
            if clicked {
                app.popout_tile("world_map");
            }
        }
        let toolbar = ui.scope_builder(egui::UiBuilder::new().max_rect(toolbar_rect), |ui| {
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
            });
        });
        // Płótno zaczyna się pod paskiem (także gdy przycisk odpinania jest wyższy od paska).
        let toolbar_bottom = toolbar
            .response
            .rect
            .bottom()
            .max(toolbar_area.min.y + if docked { 24.0 } else { 0.0 });
        ui.advance_cursor_after_rect(egui::Rect::from_min_max(
            toolbar_area.min,
            egui::pos2(toolbar_area.max.x, toolbar_bottom),
        ));

        ui.add_space(2.0);

        // Płótno Canvas Mapy Świata (Equirectangular projection)
        // Płótno wypełnia całą dostępną przestrzeń pod paskiem narzędzi (bez sztywnych
        // limitów); rzut `2:1` jest dopasowywany do niego i wyśrodkowany (letterbox).
        let avail = ui.available_size();
        let canvas_size = egui::vec2(
            if avail.x.is_finite() { avail.x } else { 640.0 },
            if avail.y.is_finite() { avail.y } else { 320.0 },
        )
        .max(egui::vec2(1.0, 1.0));

        let id = ui.id().with("world_map_state");
        let mut state = ui.data_mut(|d| d.get_temp::<MapState>(id).unwrap_or_default());

        let (response, _) = ui.allocate_painter(canvas_size, egui::Sense::click_and_drag());
        let canvas_rect = response.rect;
        // Ścisły `clip_rect`: nic z mapy nie wystaje poza kafelek.
        let painter = ui.painter_at(canvas_rect);
        // Prostokąt rzutu (proporcja 2:1) wewnątrz płótna.
        let rect = fit_map_rect(canvas_rect);
        app.world_map.last_canvas = Some(canvas_rect);
        app.world_map.last_map_rect = Some(rect);

        state.zoom = if state.zoom.is_finite() {
            state.zoom.clamp(MIN_ZOOM, MAX_ZOOM)
        } else {
            MIN_ZOOM
        };
        if response.dragged() && rect.width() > 0.0 && rect.height() > 0.0 {
            let delta = response.drag_delta();
            state.pan += egui::vec2(delta.x / rect.width(), delta.y / rect.height());
        }
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                state.zoom = (state.zoom + scroll * 0.001).clamp(MIN_ZOOM, MAX_ZOOM);
            }
        }

        // Tło oceanu wypełnia całe płótno (także pasy letterboxu).
        painter.rect_filled(canvas_rect, 4.0, map.ocean);

        // Przyciski przybliżania: w pionie przy dolnej krawędzi; na niskim płótnie w poziomie;
        // na bardzo małym kafelku ukryte (kółko myszy nadal działa).
        let canvas_side = canvas_rect.width().min(canvas_rect.height());
        if canvas_side >= ZOOM_BUTTONS_MIN_CANVAS {
            let vertical = canvas_rect.height() >= 150.0;
            let zoom_rect = if vertical {
                egui::Rect::from_min_size(
                    canvas_rect.left_bottom() + egui::vec2(10.0, -70.0),
                    egui::vec2(30.0, 60.0),
                )
            } else {
                egui::Rect::from_min_size(
                    canvas_rect.left_bottom() + egui::vec2(10.0, -30.0),
                    egui::vec2(66.0, 24.0),
                )
            };
            ui.scope_builder(egui::UiBuilder::new().max_rect(zoom_rect), |ui| {
                let mut buttons = |ui: &mut egui::Ui| {
                    if ui.button("+").clicked() {
                        state.zoom = (state.zoom + 0.5).clamp(MIN_ZOOM, MAX_ZOOM);
                    }
                    if ui.button("-").clicked() {
                        state.zoom = (state.zoom - 0.5).clamp(MIN_ZOOM, MAX_ZOOM);
                    }
                };
                if vertical {
                    ui.vertical(&mut buttons);
                } else {
                    ui.horizontal(&mut buttons);
                }
            });
        }

        // Przesunięcie zawsze mieści się w aktualnym płótnie; przy zoom 1.0 mapa jest na środku.
        state.pan = clamp_pan(state.pan, state.zoom, canvas_rect, rect);
        ui.data_mut(|d| d.insert_temp(id, state));
        let pan_px = egui::vec2(state.pan.x * rect.width(), state.pan.y * rect.height());

        // Pomocnik projekcji: (lat, lon) -> Pos2
        let project = |lat: f64, lon: f64| -> egui::Pos2 {
            let base_x = ((lon + 180.0) / 360.0) as f32 * rect.width();
            let base_y = ((90.0 - lat) / 180.0) as f32 * rect.height();
            let center_x = rect.width() / 2.0;
            let center_y = rect.height() / 2.0;
            let zoomed_x = (base_x - center_x) * state.zoom + center_x + pan_px.x;
            let zoomed_y = (base_y - center_y) * state.zoom + center_y + pan_px.y;
            egui::pos2(rect.min.x + zoomed_x, rect.min.y + zoomed_y)
        };
        let overlay = overlay_scale(canvas_rect);

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

        // QSO markers: one per locator (cached per log revision), drawn only when visible and
        // not already covered by a marker of the same colour in the same 2-pixel cell.
        app.world_map.refresh_markers(
            &app.active_journal.id,
            app.log_view.log_revision,
            &app.log_view.recent_qsos,
        );
        let visible_area = canvas_rect.expand(4.0);
        let mut drawn_cells = std::collections::HashSet::new();
        let mut marker_mesh = egui::epaint::Mesh::default();
        let outline = map.ocean.alpha(150);
        let mut hovered: Option<(egui::Pos2, egui::Color32, String)> = None;
        for marker in &app.world_map.markers {
            let qso_pos = project(marker.latitude, marker.longitude);
            if !visible_area.contains(qso_pos) {
                continue;
            }
            let color = if marker.confirmed {
                map.qso_marker_confirmed
            } else {
                map.qso_marker
            };
            let cell = (
                (qso_pos.x * 0.5) as i32,
                (qso_pos.y * 0.5) as i32,
                marker.confirmed,
            );
            if drawn_cells.insert(cell) {
                push_dot(&mut marker_mesh, qso_pos, 4.0, outline);
                push_dot(&mut marker_mesh, qso_pos, 2.5, color);
            }
            if hovered.is_none()
                && let Some(hp) = hover_pos
                && (hp - qso_pos).length() < 5.0
            {
                hovered = Some((qso_pos, color, marker.callsign.clone()));
            }
        }
        painter.add(egui::Shape::mesh(marker_mesh));
        if let Some((qso_pos, color, callsign)) = hovered {
            let tip_box =
                egui::Rect::from_min_size(qso_pos + egui::vec2(6.0, -12.0), egui::vec2(90.0, 16.0));
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
                format!("QSO: {callsign}"),
                egui::FontId::proportional(9.0),
                color,
            );
        }

        // Znacznik stacji własnej (My QTH) i punkt początkowy wiązki
        let my_pos = project(my_coords.latitude, my_coords.longitude);

        // Rysowanie wiązki promieniowania anteny (Beam Lobe) oraz mechanicznego boomu
        if app.world_map.show_beam_lobe
            && let Some(az) = beam_az
        {
            let half_beam = (app.world_map.beamwidth_deg * 0.5).clamp(10.0, 60.0);
            let lobe_len = 65.0_f32 * overlay;

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
            let boom_len = 32.0 * overlay;
            let boom_tip =
                my_pos + egui::vec2(boom_rad.cos() * boom_len, boom_rad.sin() * boom_len);
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
        // Promień zależy od rozmiaru płótna; na bardzo małym kafelku kompas jest ukryty.
        if app.world_map.show_compass
            && let Some(compass_radius) = compass_radius(canvas_rect)
        {
            let compass_center = compass_center(canvas_rect, compass_radius);
            let cardinal_inset = compass_radius * 0.2;

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
                compass_center + egui::vec2(0.0, -compass_radius + cardinal_inset),
                egui::Align2::CENTER_CENTER,
                "N",
                font_card.clone(),
                palette.danger,
            );
            painter.text(
                compass_center + egui::vec2(compass_radius - cardinal_inset, 0.0),
                egui::Align2::CENTER_CENTER,
                "E",
                font_card.clone(),
                map.compass_text,
            );
            painter.text(
                compass_center + egui::vec2(0.0, compass_radius - cardinal_inset),
                egui::Align2::CENTER_CENTER,
                "S",
                font_card.clone(),
                map.compass_text,
            );
            painter.text(
                compass_center + egui::vec2(-compass_radius + cardinal_inset, 0.0),
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
                    compass_center + egui::vec2(0.0, compass_radius * 0.33),
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
                    app.rotate_antenna_to(click_az);
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas(width: f32, height: f32) -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(13.0, 27.0), egui::vec2(width, height))
    }

    fn contains(outer: egui::Rect, inner: egui::Rect) -> bool {
        outer.expand(1e-3).contains_rect(inner)
    }

    #[test]
    fn projection_rect_is_two_to_one_centered_and_inside_every_canvas() {
        for (w, h) in [
            (300.0, 600.0),
            (1200.0, 260.0),
            (320.0, 200.0),
            (1920.0, 1080.0),
            (400.0, 200.0),
            (1.0, 1.0),
            (0.0, 0.0),
        ] {
            let c = canvas(w, h);
            let map = fit_map_rect(c);
            assert!(contains(c, map), "{w}x{h}: mapa w płótnie");
            if w > 1.0 {
                assert!((map.width() / map.height() - 2.0).abs() < 1e-3, "{w}x{h}");
            }
            assert!((map.center().x - c.center().x).abs() < 1e-3);
            assert!((map.center().y - c.center().y).abs() < 1e-3);
            // Wypełnia płótno w jednym z wymiarów (brak niepotrzebnych pustych pasów).
            assert!(
                (map.width() - c.width()).abs() < 1e-3 || (map.height() - c.height()).abs() < 1e-3,
                "{w}x{h}"
            );
        }
    }

    #[test]
    fn pan_is_clamped_and_resets_to_center_at_zoom_one() {
        let c = canvas(800.0, 400.0);
        let map = fit_map_rect(c);
        let wild = egui::vec2(9.0, -9.0);
        assert_eq!(clamp_pan(wild, 1.0, c, map), egui::Vec2::ZERO);
        assert_eq!(clamp_pan(wild, f32::NAN, c, map), egui::Vec2::ZERO);
        assert_eq!(
            clamp_pan(egui::vec2(f32::NAN, 0.2), 3.0, c, map),
            egui::Vec2::ZERO
        );
        // Przy zoom 3 przybliżona mapa (2400x1200) pokrywa płótno: pan <= (2400-800)/2/800 = 1.0.
        let limited = clamp_pan(wild, 3.0, c, map);
        assert!((limited.x - 1.0).abs() < 1e-4, "{limited:?}");
        assert!((limited.y + 1.0).abs() < 1e-4, "{limited:?}");
        let ok = egui::vec2(0.3, -0.2);
        assert_eq!(clamp_pan(ok, 3.0, c, map), ok);
    }

    #[test]
    fn pan_stays_in_frame_after_the_tile_is_resized() {
        let big = canvas(1600.0, 800.0);
        let small = canvas(320.0, 200.0);
        let mut pan = egui::vec2(1.2, 0.7);
        pan = clamp_pan(pan, 3.0, big, fit_map_rect(big));
        let map = fit_map_rect(small);
        let after = clamp_pan(pan, 3.0, small, map);
        // Przesunięta mapa nadal pokrywa płótno w obu osiach.
        let left = map.center().x + after.x * map.width() - map.width() * 3.0 / 2.0;
        let right = map.center().x + after.x * map.width() + map.width() * 3.0 / 2.0;
        assert!(left <= small.left() + 1e-2 && right >= small.right() - 1e-2);
        let top = map.center().y + after.y * map.height() - map.height() * 3.0 / 2.0;
        let bottom = map.center().y + after.y * map.height() + map.height() * 3.0 / 2.0;
        assert!(top <= small.top() + 1e-2 && bottom >= small.bottom() - 1e-2);
        // Powrót do zoom 1.0 = idealnie na środku.
        assert_eq!(
            clamp_pan(after, 1.0, small, map),
            egui::Vec2::ZERO,
            "zoom 1.0 centruje mapę"
        );
    }

    #[test]
    fn compass_scales_with_canvas_and_stays_inside() {
        assert_eq!(compass_radius(canvas(320.0, 120.0)), None);
        assert_eq!(compass_radius(canvas(149.0, 900.0)), None);
        let mut previous = 0.0;
        for side in [150.0_f32, 250.0, 330.0, 600.0, 2000.0] {
            let c = canvas(side * 2.0, side);
            let radius = compass_radius(c).expect("kompas widoczny");
            assert!((26.0..=48.0).contains(&radius), "{side}: {radius}");
            assert!(radius >= previous);
            previous = radius;
            let center = compass_center(c, radius);
            let disc = egui::Rect::from_center_size(center, egui::Vec2::splat(radius * 2.0));
            assert!(contains(c, disc), "{side}: tarcza w płótnie");
            // Tarcza nie zasłania więcej niż ułamek mniejszego boku.
            assert!(radius * 2.0 <= side * 0.5, "{side}");
        }
    }

    #[test]
    fn overlay_scale_is_bounded() {
        assert_eq!(overlay_scale(canvas(100.0, 80.0)), 0.5);
        assert_eq!(overlay_scale(canvas(3000.0, 2000.0)), 1.0);
        let mid = overlay_scale(canvas(600.0, 300.0));
        assert!((0.5..=1.0).contains(&mid));
    }
}
