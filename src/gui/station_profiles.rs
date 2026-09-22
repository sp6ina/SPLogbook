// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use crate::core::i18n::tr;
use crate::core::station::StationProfile;
use crate::gui::app::SpLogApp;
use eframe::egui;

pub fn render_station_profiles_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_station_profiles_window {
        return;
    }

    let lang = app.current_language;
    let mut is_open = app.show_station_profiles_window;
    let mut close_requested = false;
    let mut need_save = false;

    if app.station_profiles.is_empty() {
        let mut def = app.my_station.clone();
        if def.id.is_empty() {
            def.id = "default".to_string();
        }
        if def.name.is_empty() {
            def.name = tr("profiles.new_default_name", lang).to_string();
        }
        app.station_profiles.push(def);
        app.active_profile_id = "default".to_string();
    }

    egui::Window::new(egui::RichText::new(tr("profiles.title", lang)).size(14.0).strong())
        .open(&mut is_open)
        .default_size([680.0, 480.0])
        .min_size([520.0, 360.0])
        .resizable(true)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(tr("profiles.desc", lang)).small().color(egui::Color32::from_rgb(148, 163, 184)));
                ui.separator();

                ui.columns(2, |cols| {
                    cols[0].group(|ui| {
                        ui.label(egui::RichText::new(tr("profiles.available", lang)).strong());
                        ui.add_space(4.0);

                        let mut active_to_switch = None;
                        let mut delete_idx = None;
                        let mut clone_profile = None;

                        egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                            for (idx, prof) in app.station_profiles.iter().enumerate() {
                                let is_active = app.active_profile_id == prof.id;
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        if is_active {
                                            ui.label(egui::RichText::new(tr("profiles.active_badge", lang)).color(egui::Color32::from_rgb(34, 197, 94)).strong().size(10.0));
                                        } else if ui.button(egui::RichText::new(tr("profiles.activate_btn", lang)).size(10.0)).clicked() {
                                            active_to_switch = Some(prof.id.clone());
                                        }

                                        let display_name = if prof.name.is_empty() { &prof.callsign } else { &prof.name };
                                        ui.label(egui::RichText::new(display_name).strong());
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(format!("{}: {} | QTH: {}", tr("qso.callsign", lang), prof.callsign, prof.gridsquare)).size(10.0).color(egui::Color32::GRAY));
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if app.station_profiles.len() > 1 {
                                                if ui.small_button("🗑").on_hover_text(tr("profiles.delete_tip", lang)).clicked() {
                                                    delete_idx = Some(idx);
                                                }
                                            }
                                            if ui.small_button("📋").on_hover_text(tr("profiles.clone_tip", lang)).clicked() {
                                                clone_profile = Some(prof.clone());
                                            }
                                        });
                                    });
                                });
                                ui.add_space(2.0);
                            }
                        });

                        if let Some(new_id) = active_to_switch {
                            app.activate_station_profile(&new_id);
                        }

                        if let Some(idx) = delete_idx {
                            let was_active = app.station_profiles[idx].id == app.active_profile_id;
                            app.station_profiles.remove(idx);
                            if was_active && !app.station_profiles.is_empty() {
                                let next_id = app.station_profiles[0].id.clone();
                                app.activate_station_profile(&next_id);
                            }
                            need_save = true;
                        }

                        if let Some(mut to_clone) = clone_profile {
                            let new_id = format!("prof-{}", chrono::Utc::now().timestamp_millis());
                            to_clone.id = new_id;
                            to_clone.name = format!("{} (2)", to_clone.name);
                            app.station_profiles.push(to_clone);
                            need_save = true;
                        }

                        ui.separator();
                        if ui.button(egui::RichText::new(tr("profiles.add_btn", lang)).color(egui::Color32::from_rgb(56, 189, 248)).strong()).clicked() {
                            let new_id = format!("prof-{}", chrono::Utc::now().timestamp_millis());
                            let new_prof = StationProfile {
                                id: new_id,
                                name: tr("profiles.new_default_name", lang).to_string(),
                                callsign: app.my_station.callsign.clone(),
                                operator: app.my_station.operator.clone(),
                                gridsquare: app.my_station.gridsquare.clone(),
                                city: app.my_station.city.clone(),
                                country: app.my_station.country.clone(),
                                itu_zone: app.my_station.itu_zone,
                                cq_zone: app.my_station.cq_zone,
                                pga_gmina: None,
                                sota_ref: None,
                                pota_ref: None,
                                power_watts: 100,
                                default_rig: None,
                                default_antenna: None,
                            };
                            app.station_profiles.push(new_prof);
                            need_save = true;
                        }
                    });

                    cols[1].group(|ui| {
                        let active_id = app.active_profile_id.clone();
                        if let Some(prof) = app.station_profiles.iter_mut().find(|p| p.id == active_id) {
                            ui.label(egui::RichText::new(tr("profiles.edit_header", lang)).strong().color(egui::Color32::from_rgb(56, 189, 248)));
                            ui.add_space(4.0);

                            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(tr("profiles.name_label", lang));
                                    if ui.add(egui::TextEdit::singleline(&mut prof.name).hint_text("Home QTH, /P, SOTA")).changed() {
                                        need_save = true;
                                    }
                                });

                                ui.horizontal(|ui| {
                                    ui.label(format!("{}:", tr("qso.callsign", lang)));
                                    if ui.add(egui::TextEdit::singleline(&mut prof.callsign).desired_width(120.0)).changed() {
                                        need_save = true;
                                    }
                                    ui.label(format!("{}:", tr("qso.name", lang)));
                                    if ui.add(egui::TextEdit::singleline(&mut prof.operator).desired_width(120.0)).changed() {
                                        need_save = true;
                                    }
                                });

                                ui.horizontal(|ui| {
                                    ui.label(format!("{}:", tr("qso.locator", lang)));
                                    if ui.add(egui::TextEdit::singleline(&mut prof.gridsquare).desired_width(90.0)).changed() {
                                        need_save = true;
                                    }
                                    ui.label(format!("{}:", tr("qso.power", lang)));
                                    if ui.add(egui::DragValue::new(&mut prof.power_watts).range(1..=1500)).changed() {
                                        need_save = true;
                                    }
                                });

                                ui.horizontal(|ui| {
                                    ui.label(format!("{}:", tr("qso.qth", lang)));
                                    if ui.add(egui::TextEdit::singleline(&mut prof.city).desired_width(110.0)).changed() {
                                        need_save = true;
                                    }
                                    ui.label(format!("{}:", tr("geo.country", lang)));
                                    if ui.add(egui::TextEdit::singleline(&mut prof.country).desired_width(110.0)).changed() {
                                        need_save = true;
                                    }
                                });

                                ui.horizontal(|ui| {
                                    ui.label(format!("{}:", tr("geo.cq_zone", lang)));
                                    if ui.add(egui::DragValue::new(&mut prof.cq_zone).range(1..=40)).changed() {
                                        need_save = true;
                                    }
                                    ui.label(format!("{}:", tr("geo.itu_zone", lang)));
                                    if ui.add(egui::DragValue::new(&mut prof.itu_zone).range(1..=90)).changed() {
                                        need_save = true;
                                    }
                                });

                                ui.separator();
                                ui.label(egui::RichText::new(tr("profiles.awards_section", lang)).small().color(egui::Color32::from_rgb(250, 204, 21)));

                                ui.horizontal(|ui| {
                                    ui.label(format!("{}:", tr("qso.pga", lang)));
                                    let mut pga_str = prof.pga_gmina.clone().unwrap_or_default();
                                    if ui.add(egui::TextEdit::singleline(&mut pga_str).desired_width(80.0).hint_text("OP01")).changed() {
                                        prof.pga_gmina = if pga_str.trim().is_empty() { None } else { Some(pga_str.trim().to_uppercase()) };
                                        need_save = true;
                                    }
                                });

                                ui.horizontal(|ui| {
                                    ui.label(format!("{}:", tr("qso.sota", lang)));
                                    let mut sota_str = prof.sota_ref.clone().unwrap_or_default();
                                    if ui.add(egui::TextEdit::singleline(&mut sota_str).desired_width(90.0).hint_text("SP/BZ-001")).changed() {
                                        prof.sota_ref = if sota_str.trim().is_empty() { None } else { Some(sota_str.trim().to_uppercase()) };
                                        need_save = true;
                                    }

                                    ui.label(format!("{}:", tr("qso.pota", lang)));
                                    let mut pota_str = prof.pota_ref.clone().unwrap_or_default();
                                    if ui.add(egui::TextEdit::singleline(&mut pota_str).desired_width(80.0).hint_text("SP-0123")).changed() {
                                        prof.pota_ref = if pota_str.trim().is_empty() { None } else { Some(pota_str.trim().to_uppercase()) };
                                        need_save = true;
                                    }
                                });
                            });
                        }
                    });
                });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new(tr("btn.save", lang)).strong()).clicked() {
                        need_save = true;
                        close_requested = true;
                    }
                    if ui.button(tr("btn.close", lang)).clicked() {
                        close_requested = true;
                    }
                });
            });
        });

    if close_requested {
        is_open = false;
    }
    app.show_station_profiles_window = is_open;

    if need_save {
        let active_id = app.active_profile_id.clone();
        if let Some(prof) = app.station_profiles.iter().find(|p| p.id == active_id) {
            app.my_station = prof.clone();
        }
        app.save_station_config();
    }
}
