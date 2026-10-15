// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use eframe::egui;

use crate::cat::config::{
    CatStationConfig, ElecraftBackendConfig, ElecraftTransportConfig, FlexRadioBackendConfig,
    FlexRadioConnectionConfig, FlrigBackendConfig, HamlibBackendConfig, IcomBackendConfig,
    KenwoodBackendConfig, KenwoodTransportConfig, RigBackendConfig, RigConfig, RigTargetConfig,
    TciBackendConfig, YaesuBackendConfig,
};
use crate::cat::probe::RigProbeRunner;
use crate::cat::rig_models::search_rig_models_by_mfg;
use crate::cat::routing::RigId;
use crate::cat::service::RigServiceState;
use crate::cat::tx_arbiter::{TxArbiterState, redact_sensitive_info};
use crate::core::i18n::{key, tr};
use crate::gui::app::SpLogApp;

pub fn render_cat_settings_window(app: &mut SpLogApp, ctx: &egui::Context) {
    if !app.show_cat_settings_window {
        return;
    }

    let lang = app.current_language;
    let mut is_open = app.show_cat_settings_window;
    let mut close_req = false;

    // 1. Upewnij się, że cat_station istnieje i zawiera co najmniej jeden wpis
    if app.cat_station.is_none() {
        let mut temp_cfg = app.build_app_config();
        let _ = crate::cat::config::migrate_legacy_cat_config(&mut temp_cfg);
        app.cat_station = temp_cfg.cat_station;
    }
    if app.cat_station.is_none() {
        app.cat_station = Some(CatStationConfig::default());
    }
    let mut station_cfg = app.cat_station.take().unwrap_or_default();
    if station_cfg.rigs.is_empty() {
        let default_id = RigId::new("rig-1").unwrap_or_else(|_| RigId::hub());
        station_cfg.rigs.push(RigConfig {
            id: default_id.clone(),
            enabled: true,
            display_name: "Transceiver 1".to_string(),
            poll_interval_ms: 250,
            command_timeout_ms: 2000,
            backend: RigBackendConfig::Hamlib(HamlibBackendConfig::default()),
        });
        station_cfg.active_rx = Some(RigTargetConfig::rig_only(default_id));
    }

    let current_idx = app.cat_selected_rig_idx.min(station_cfg.rigs.len() - 1);

    // Snapshot z CatRuntime dla stanu cyklu życia i telemetrii
    let cat_snap = app.cat_runtime.snapshot();
    let selected_rig_id = station_cfg.rigs[current_idx].id.clone();
    let svc_snap = cat_snap.rigs.get(&selected_rig_id).cloned();

    egui::Window::new(
        egui::RichText::new(format!("📻 {}", tr(key::CAT_SETTINGS_TITLE, lang)))
            .size(14.0)
            .strong(),
    )
    .open(&mut is_open)
    .default_size([720.0, 640.0])
    .min_size([520.0, 480.0])
    .resizable(true)
    .show(ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.vertical(|ui| {
                {
                    let rig = &mut station_cfg.rigs[current_idx];

                    // Pasek statusu połączenia i cyklu życia (Lifecycle Status Bar)
                    render_lifecycle_status_bar(app, ui, rig, svc_snap.as_ref(), &cat_snap, lang);

                    ui.separator();

                    // Krok 1: Wybór backendu (Rodzaj integracji)
                    render_backend_kind_selector(app, ui, rig, lang);

                    ui.add_space(4.0);

                    // Krok 2: Dynamiczny formularz specyficzny dla wybranego backendu
                    render_backend_form(app, ui, rig, lang);

                    ui.add_space(4.0);

                    // Krok 3: Parametry odpytywania (Poll Rate)
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}:", tr(key::CAT_POLL_RATE, lang)));
                            ui.add(
                                egui::DragValue::new(&mut rig.poll_interval_ms)
                                    .range(50..=2000)
                                    .suffix(" ms"),
                            );
                            ui.label(
                                egui::RichText::new("(50 - 2000 ms, domyślnie 250 ms)")
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(148, 163, 184)),
                            );
                        });
                    });

                    ui.add_space(4.0);

                    // Krok 4: Bezpieczny, Read-Only Probe
                    render_probe_section(app, ui, rig, lang);
                }

                ui.add_space(4.0);

                // Krok 5: Prezentacja RigCapabilities i Diagnostyki
                render_capabilities_and_diagnostics(
                    ui,
                    svc_snap.as_ref(),
                    app.cat_probe_result.as_ref(),
                    lang,
                );

                ui.add_space(4.0);

                // Krok 6: CAT Sharing (Hamlib Proxy Server)
                render_cat_sharing_section(app, ui, lang);

                ui.add_space(4.0);

                // Krok 7: Sterowanie rotorem (rotctld)
                render_rotor_section(app, ui, lang);

                ui.separator();

                // Stopka z przyciskami Zapisz / Zamknij
                ui.horizontal(|ui| {
                    if ui
                        .button(egui::RichText::new(tr(key::BTN_SAVE, lang)).strong())
                        .clicked()
                    {
                        if let Err(e) = station_cfg.validate() {
                            app.cat_test_result = Some(format!(
                                "{}: {e}",
                                tr(key::CAT_SETTINGS_VALIDATION_FAILED, lang)
                            ));
                        } else {
                            // Synchronizacja legacy pól w app dla kompatybilności wstecznej
                            sync_legacy_fields_from_rig(app, &station_cfg.rigs[current_idx]);
                            if app.cat_connected || app.cat_auto_start_rigctld {
                                app.start_cat_service();
                            }
                            close_req = true;
                        }
                    }
                    if ui.button(tr(key::BTN_CLOSE, lang)).clicked() {
                        close_req = true;
                    }
                });
            });
        });
    });

    if close_req {
        is_open = false;
        app.cat_station = Some(station_cfg);
        app.save_station_config();
    } else {
        app.cat_station = Some(station_cfg);
    }
    app.show_cat_settings_window = is_open;
}

/// Renderuje pasek stanu cyklu życia sesji ([`RigServiceState`]) oraz arbitrażu nadawania ([`TxArbiterState`]).
fn render_lifecycle_status_bar(
    app: &mut SpLogApp,
    ui: &mut egui::Ui,
    rig: &RigConfig,
    svc_snap: Option<&crate::cat::service::RigServiceSnapshot>,
    cat_snap: &crate::cat::hub::RigHubSnapshot,
    lang: crate::core::i18n::Language,
) {
    ui.horizontal(|ui| {
        // 1. Stan połączenia i usługi
        let (color, text) = if let Some(svc) = svc_snap {
            match svc.lifecycle {
                RigServiceState::Ready => (
                    egui::Color32::from_rgb(34, 197, 94),
                    tr(key::CAT_SETTINGS_STATE_CONNECTED, lang),
                ),
                RigServiceState::Connecting => (
                    egui::Color32::from_rgb(250, 204, 21),
                    tr(key::CAT_SETTINGS_STATE_CONNECTING, lang),
                ),
                RigServiceState::Degraded => (
                    egui::Color32::from_rgb(251, 146, 60),
                    tr(key::CAT_SETTINGS_STATE_DEGRADED, lang),
                ),
                RigServiceState::Disconnecting => {
                    (egui::Color32::from_rgb(148, 163, 184), "Rozłączanie...")
                }
                RigServiceState::Synchronizing => {
                    (egui::Color32::from_rgb(56, 189, 248), "Synchronizacja...")
                }
                RigServiceState::Disconnected | RigServiceState::Created => (
                    egui::Color32::from_rgb(148, 163, 184),
                    tr(key::CAT_SETTINGS_STATE_DISCONNECTED, lang),
                ),
                RigServiceState::Failed => (
                    egui::Color32::from_rgb(239, 68, 68),
                    tr(key::CAT_SETTINGS_STATE_FAILED, lang),
                ),
            }
        } else if app.cat_connected {
            (
                egui::Color32::from_rgb(34, 197, 94),
                tr(key::CAT_SETTINGS_STATE_CONNECTED, lang),
            )
        } else {
            (
                egui::Color32::from_rgb(148, 163, 184),
                tr(key::CAT_SETTINGS_STATE_DISCONNECTED, lang),
            )
        };

        ui.colored_label(color, format!("● {text}"));

        if app.cat_connected {
            if ui.button(tr(key::CAT_DISCONNECT_RADIO, lang)).clicked() {
                app.stop_cat_service();
            }
        } else if ui.button(tr(key::CAT_CONNECT_RADIO, lang)).clicked() {
            app.start_cat_service();
        }

        ui.separator();

        // 2. Prezentacja stanu arbitra TX
        ui.label(tr(key::CAT_SETTINGS_TX_ARBITER_STATUS, lang));
        match cat_snap.tx_state {
            TxArbiterState::Idle => {
                ui.colored_label(
                    egui::Color32::from_rgb(34, 197, 94),
                    tr(key::CAT_SETTINGS_TX_IDLE, lang),
                );
            }
            TxArbiterState::Requesting { .. } => {
                ui.colored_label(egui::Color32::from_rgb(250, 204, 21), "Requesting...");
            }
            TxArbiterState::Owned { ref rig_id, .. } => {
                let is_current = rig_id == &rig.id;
                let color = if is_current {
                    egui::Color32::from_rgb(239, 68, 68)
                } else {
                    egui::Color32::from_rgb(251, 146, 60)
                };
                ui.colored_label(
                    color,
                    format!("{}: {rig_id}", tr(key::CAT_SETTINGS_TX_OWNED, lang)),
                );
            }
            TxArbiterState::Releasing { .. } => {
                ui.colored_label(egui::Color32::from_rgb(250, 204, 21), "Releasing...");
            }
            TxArbiterState::Unknown { ref reason, .. } => {
                ui.colored_label(
                    egui::Color32::from_rgb(239, 68, 68),
                    format!("⚠ {} ({reason})", tr(key::CAT_SETTINGS_TX_UNKNOWN, lang)),
                );
                if ui
                    .button(
                        egui::RichText::new(tr(key::CAT_SETTINGS_TX_RESYNC_BTN, lang))
                            .color(egui::Color32::from_rgb(250, 204, 21))
                            .strong(),
                    )
                    .clicked()
                {
                    app.cat_runtime
                        .resync_tx(rig.id.clone(), std::time::Duration::from_millis(1500));
                }
            }
        }
    });
}

/// Renderuje wybór rodzaju backendu CAT.
fn render_backend_kind_selector(
    app: &mut SpLogApp,
    ui: &mut egui::Ui,
    rig: &mut RigConfig,
    lang: crate::core::i18n::Language,
) {
    ui.group(|ui| {
        ui.label(
            egui::RichText::new(tr(key::CAT_SETTINGS_STEP1_TITLE, lang))
                .strong()
                .color(egui::Color32::from_rgb(250, 204, 21)),
        );
        ui.add_space(2.0);

        ui.horizontal_wrapped(|ui| {
            let backends = [
                ("hamlib", tr(key::CAT_SETTINGS_BACKEND_HAMLIB, lang)),
                ("flrig", tr(key::CAT_SETTINGS_BACKEND_FLRIG, lang)),
                ("tci", tr(key::CAT_SETTINGS_BACKEND_TCI, lang)),
                ("icom", tr(key::CAT_SETTINGS_BACKEND_ICOM, lang)),
                ("kenwood", tr(key::CAT_SETTINGS_BACKEND_KENWOOD, lang)),
                ("yaesu", tr(key::CAT_SETTINGS_BACKEND_YAESU, lang)),
                ("elecraft", tr(key::CAT_SETTINGS_BACKEND_ELECRAFT, lang)),
                ("flexradio", tr(key::CAT_SETTINGS_BACKEND_FLEXRADIO, lang)),
            ];

            let current_kind = rig.backend.kind_str();

            for (tag, label) in backends {
                let is_sel = current_kind == tag;
                if ui.selectable_label(is_sel, label).clicked() && !is_sel {
                    rig.backend = match tag {
                        "hamlib" => RigBackendConfig::Hamlib(HamlibBackendConfig::default()),
                        "flrig" => RigBackendConfig::Flrig(FlrigBackendConfig::default()),
                        "tci" => RigBackendConfig::Tci(TciBackendConfig::default()),
                        "icom" => RigBackendConfig::Icom(IcomBackendConfig::default()),
                        "kenwood" => RigBackendConfig::Kenwood(KenwoodBackendConfig::default()),
                        "yaesu" => RigBackendConfig::Yaesu(YaesuBackendConfig::default()),
                        "elecraft" => RigBackendConfig::Elecraft(ElecraftBackendConfig::default()),
                        "flexradio" => {
                            RigBackendConfig::FlexRadio(FlexRadioBackendConfig::default())
                        }
                        _ => RigBackendConfig::Hamlib(HamlibBackendConfig::default()),
                    };
                    app.cat_backend = tag.to_string();
                }
            }
        });
    });
}

/// Renderuje dynamiczny formularz w zależności od wybranego backendu.
fn render_backend_form(
    app: &mut SpLogApp,
    ui: &mut egui::Ui,
    rig: &mut RigConfig,
    lang: crate::core::i18n::Language,
) {
    ui.group(|ui| {
        ui.label(
            egui::RichText::new(tr(key::CAT_SETTINGS_STEP2_TITLE, lang))
                .strong()
                .color(egui::Color32::from_rgb(250, 204, 21)),
        );
        ui.add_space(2.0);

        match &mut rig.backend {
            RigBackendConfig::Hamlib(cfg) => {
                render_hamlib_form(app, ui, cfg, lang);
            }
            RigBackendConfig::Flrig(cfg) => {
                render_flrig_form(ui, cfg);
            }
            RigBackendConfig::Tci(cfg) => {
                render_tci_form(ui, cfg);
            }
            RigBackendConfig::Icom(cfg) => {
                render_icom_form(ui, cfg, lang);
            }
            RigBackendConfig::Kenwood(cfg) => {
                render_kenwood_form(ui, cfg, lang);
            }
            RigBackendConfig::Yaesu(cfg) => {
                render_yaesu_form(ui, cfg, lang);
            }
            RigBackendConfig::Elecraft(cfg) => {
                render_elecraft_form(ui, cfg, lang);
            }
            RigBackendConfig::FlexRadio(cfg) => {
                render_flexradio_form(ui, cfg, lang);
            }
        }
    });
}

fn render_hamlib_form(
    app: &mut SpLogApp,
    ui: &mut egui::Ui,
    cfg: &mut HamlibBackendConfig,
    lang: crate::core::i18n::Language,
) {
    ui.horizontal(|ui| {
        ui.label(tr(key::CAT_SETTINGS_CONN_TYPE, lang));
        let is_serial = app.cat_conn_type == "serial";
        if ui
            .selectable_label(is_serial, tr(key::CAT_SETTINGS_CONN_SERIAL, lang))
            .clicked()
        {
            app.cat_conn_type = "serial".to_string();
            app.cat_auto_start_rigctld = true;
        }
        if ui
            .selectable_label(!is_serial, tr(key::CAT_SETTINGS_CONN_TCP, lang))
            .clicked()
        {
            app.cat_conn_type = "tcp".to_string();
        }
    });

    ui.separator();

    if app.cat_conn_type == "tcp" {
        ui.horizontal(|ui| {
            ui.label("Host TCP:");
            ui.add(egui::TextEdit::singleline(&mut cfg.host).desired_width(120.0));
            ui.add_space(10.0);
            ui.label("Port TCP:");
            let mut port_str = cfg.port.to_string();
            if ui
                .add(egui::TextEdit::singleline(&mut port_str).desired_width(60.0))
                .changed()
            {
                if let Ok(p) = port_str.trim().parse::<u16>() {
                    cfg.port = p;
                }
            }
        });
    } else {
        render_serial_port_selector(ui, &mut app.cat_serial_port, &mut app.cat_baud_rate, lang);

        ui.horizontal(|ui| {
            ui.checkbox(
                &mut app.cat_auto_start_rigctld,
                tr(key::CAT_SETTINGS_AUTO_START_RIGCTLD, lang),
            )
            .on_hover_text(tr(key::CAT_SETTINGS_AUTO_START_RIGCTLD_TOOLTIP, lang));
        });

        if app.cat_auto_start_rigctld {
            render_rigctld_supervisor_controls(app, ui, lang);
        }
    }

    // Wyszukiwarka modelu radia dla Hamlib
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(tr(key::CAT_SETTINGS_SEARCH_MODEL, lang));
        ui.add(
            egui::TextEdit::singleline(&mut app.cat_model_search)
                .hint_text("7300, FT-891, TS-590, G90, K4"),
        );
    });

    let filtered_rigs = search_rig_models_by_mfg("", &app.cat_model_search);
    egui::ScrollArea::vertical()
        .max_height(80.0)
        .show(ui, |ui| {
            for m in filtered_rigs {
                let is_selected = app.cat_rig_id == m.rig_id;
                let label_text = format!("[ID {:4}] {:15} - {:20}", m.rig_id, m.mfg, m.model);
                if ui.selectable_label(is_selected, label_text).clicked() {
                    app.cat_rig_id = m.rig_id;
                    app.cat_rig_model = format!("{} {}", m.mfg, m.model);
                    app.cat_baud_rate = m.default_baud;
                }
            }
        });

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!("{}:", tr(key::CAT_RIG_MODEL, lang))).strong());
        ui.colored_label(
            egui::Color32::from_rgb(56, 189, 248),
            format!("{} (ID: {})", app.cat_rig_model, app.cat_rig_id),
        );
    });
}

fn render_flrig_form(ui: &mut egui::Ui, cfg: &mut FlrigBackendConfig) {
    ui.horizontal(|ui| {
        ui.label("FLRig Host XML-RPC:");
        ui.add(egui::TextEdit::singleline(&mut cfg.host).desired_width(120.0));
        ui.add_space(10.0);
        ui.label("Port:");
        let mut port_str = cfg.port.to_string();
        if ui
            .add(egui::TextEdit::singleline(&mut port_str).desired_width(60.0))
            .changed()
        {
            if let Ok(p) = port_str.trim().parse::<u16>() {
                cfg.port = p;
            }
        }
        ui.label(
            egui::RichText::new("(domyślnie :12345)")
                .size(11.0)
                .color(egui::Color32::from_rgb(148, 163, 184)),
        );
    });
}

fn render_tci_form(ui: &mut egui::Ui, cfg: &mut TciBackendConfig) {
    ui.horizontal(|ui| {
        ui.label("TCI Host WebSocket:");
        ui.add(egui::TextEdit::singleline(&mut cfg.host).desired_width(120.0));
        ui.add_space(10.0);
        ui.label("Port:");
        let mut port_str = cfg.port.to_string();
        if ui
            .add(egui::TextEdit::singleline(&mut port_str).desired_width(60.0))
            .changed()
        {
            if let Ok(p) = port_str.trim().parse::<u16>() {
                cfg.port = p;
            }
        }
        ui.label(
            egui::RichText::new("(domyślnie :40001)")
                .size(11.0)
                .color(egui::Color32::from_rgb(148, 163, 184)),
        );
    });
}

fn render_icom_form(
    ui: &mut egui::Ui,
    cfg: &mut IcomBackendConfig,
    lang: crate::core::i18n::Language,
) {
    render_serial_port_selector(ui, &mut cfg.port_name, &mut cfg.baud_rate, lang);

    ui.horizontal(|ui| {
        ui.label(tr(key::CAT_SETTINGS_CIV_ADDRESS, lang));
        let mut civ_str = format!("0x{:02X}", cfg.civ_address);
        if ui
            .add(egui::TextEdit::singleline(&mut civ_str).desired_width(60.0))
            .changed()
        {
            let clean = civ_str
                .trim()
                .trim_start_matches("0x")
                .trim_start_matches("0X");
            if let Ok(val) = u8::from_str_radix(clean, 16) {
                cfg.civ_address = val;
            }
        }
        ui.add_space(10.0);
        ui.label("Model Icom:");
        ui.add(egui::TextEdit::singleline(&mut cfg.model).desired_width(120.0));
    });
}

fn render_kenwood_form(
    ui: &mut egui::Ui,
    cfg: &mut KenwoodBackendConfig,
    lang: crate::core::i18n::Language,
) {
    ui.horizontal(|ui| {
        ui.label(tr(key::CAT_SETTINGS_TRANSPORT_TYPE, lang));
        let is_serial = matches!(cfg.transport, KenwoodTransportConfig::Serial { .. });
        if ui
            .selectable_label(is_serial, tr(key::CAT_SETTINGS_TRANSPORT_SERIAL, lang))
            .clicked()
            && !is_serial
        {
            cfg.transport = KenwoodTransportConfig::Serial {
                port_name: "COM1".to_string(),
                baud_rate: 115200,
            };
        }
        if ui
            .selectable_label(!is_serial, tr(key::CAT_SETTINGS_TRANSPORT_LAN, lang))
            .clicked()
            && is_serial
        {
            cfg.transport = KenwoodTransportConfig::Lan {
                host: "127.0.0.1".to_string(),
                port: 60000,
            };
        }
    });

    match cfg.transport {
        KenwoodTransportConfig::Serial {
            ref mut port_name,
            ref mut baud_rate,
        } => {
            render_serial_port_selector(ui, port_name, baud_rate, lang);
        }
        KenwoodTransportConfig::Lan {
            ref mut host,
            ref mut port,
        } => {
            ui.horizontal(|ui| {
                ui.label("KNS Host:");
                ui.add(egui::TextEdit::singleline(host).desired_width(120.0));
                ui.add_space(10.0);
                ui.label("Port:");
                let mut port_str = port.to_string();
                if ui
                    .add(egui::TextEdit::singleline(&mut port_str).desired_width(60.0))
                    .changed()
                {
                    if let Ok(p) = port_str.trim().parse::<u16>() {
                        *port = p;
                    }
                }
            });
        }
    }

    ui.horizontal(|ui| {
        ui.label("Model Kenwood:");
        ui.add(egui::TextEdit::singleline(&mut cfg.model).desired_width(140.0));
    });
}

fn render_yaesu_form(
    ui: &mut egui::Ui,
    cfg: &mut YaesuBackendConfig,
    lang: crate::core::i18n::Language,
) {
    render_serial_port_selector(ui, &mut cfg.port_name, &mut cfg.baud_rate, lang);

    ui.horizontal(|ui| {
        ui.label("Model Yaesu:");
        ui.add(egui::TextEdit::singleline(&mut cfg.model).desired_width(140.0));
    });
}

fn render_elecraft_form(
    ui: &mut egui::Ui,
    cfg: &mut ElecraftBackendConfig,
    lang: crate::core::i18n::Language,
) {
    ui.horizontal(|ui| {
        ui.label(tr(key::CAT_SETTINGS_TRANSPORT_TYPE, lang));
        let is_serial = matches!(cfg.transport, ElecraftTransportConfig::Serial { .. });
        if ui
            .selectable_label(is_serial, tr(key::CAT_SETTINGS_TRANSPORT_SERIAL, lang))
            .clicked()
            && !is_serial
        {
            cfg.transport = ElecraftTransportConfig::Serial {
                port_name: "COM1".to_string(),
                baud_rate: 38400,
            };
        }
        if ui
            .selectable_label(!is_serial, tr(key::CAT_SETTINGS_TRANSPORT_LAN, lang))
            .clicked()
            && is_serial
        {
            cfg.transport = ElecraftTransportConfig::Tcp {
                host: "127.0.0.1".to_string(),
                port: 9200,
            };
        }
    });

    match cfg.transport {
        ElecraftTransportConfig::Serial {
            ref mut port_name,
            ref mut baud_rate,
        } => {
            render_serial_port_selector(ui, port_name, baud_rate, lang);
        }
        ElecraftTransportConfig::Tcp {
            ref mut host,
            ref mut port,
        } => {
            ui.horizontal(|ui| {
                ui.label("K4 Host TCP:");
                ui.add(egui::TextEdit::singleline(host).desired_width(120.0));
                ui.add_space(10.0);
                ui.label("Port:");
                let mut port_str = port.to_string();
                if ui
                    .add(egui::TextEdit::singleline(&mut port_str).desired_width(60.0))
                    .changed()
                {
                    if let Ok(p) = port_str.trim().parse::<u16>() {
                        *port = p;
                    }
                }
            });
        }
    }

    ui.horizontal(|ui| {
        ui.label("Model Elecraft:");
        ui.add(egui::TextEdit::singleline(&mut cfg.model).desired_width(140.0));
    });
}

fn render_flexradio_form(
    ui: &mut egui::Ui,
    cfg: &mut FlexRadioBackendConfig,
    lang: crate::core::i18n::Language,
) {
    ui.horizontal(|ui| {
        ui.label("Połączenie FlexRadio:");
        let is_auto = matches!(cfg.connection, FlexRadioConnectionConfig::AutoDiscovery);
        if ui
            .selectable_label(is_auto, tr(key::CAT_SETTINGS_FLEX_DISCOVERY, lang))
            .clicked()
            && !is_auto
        {
            cfg.connection = FlexRadioConnectionConfig::AutoDiscovery;
        }
        if ui
            .selectable_label(!is_auto, tr(key::CAT_SETTINGS_FLEX_DIRECT, lang))
            .clicked()
            && is_auto
        {
            cfg.connection = FlexRadioConnectionConfig::Manual {
                host: "127.0.0.1".to_string(),
                port: 4992,
            };
        }
    });

    if let FlexRadioConnectionConfig::Manual {
        ref mut host,
        ref mut port,
    } = cfg.connection
    {
        ui.horizontal(|ui| {
            ui.label("Host:");
            ui.add(egui::TextEdit::singleline(host).desired_width(120.0));
            ui.add_space(10.0);
            ui.label("Port TCP:");
            let mut port_str = port.to_string();
            if ui
                .add(egui::TextEdit::singleline(&mut port_str).desired_width(60.0))
                .changed()
            {
                if let Ok(p) = port_str.trim().parse::<u16>() {
                    *port = p;
                }
            }
        });
    }

    ui.horizontal(|ui| {
        ui.label(tr(key::CAT_SETTINGS_FLEX_SLICE, lang));
        let mut slice_val = cfg.slice_id.unwrap_or(0);
        if ui
            .add(egui::DragValue::new(&mut slice_val).range(0..=7))
            .changed()
        {
            cfg.slice_id = Some(slice_val);
        }
        ui.add_space(10.0);
        ui.label("Model:");
        let mut model_str = cfg.model.clone().unwrap_or_default();
        if ui
            .add(egui::TextEdit::singleline(&mut model_str).desired_width(100.0))
            .changed()
        {
            cfg.model = if model_str.trim().is_empty() {
                None
            } else {
                Some(model_str.trim().to_string())
            };
        }
    });
}

fn render_serial_port_selector(
    ui: &mut egui::Ui,
    port_name: &mut String,
    baud_rate: &mut u32,
    lang: crate::core::i18n::Language,
) {
    ui.horizontal(|ui| {
        ui.label(tr(key::CAT_SETTINGS_SERIAL_PORT_LABEL, lang));
        ui.add(
            egui::TextEdit::singleline(port_name)
                .desired_width(95.0)
                .hint_text("COM3, /dev/ttyUSB0"),
        );
        let port_presets: &[&str] = if cfg!(target_os = "windows") {
            &[
                "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "COM10",
                "COM11", "COM12",
            ]
        } else {
            &[
                "/dev/ttyUSB0",
                "/dev/ttyUSB1",
                "/dev/ttyACM0",
                "/dev/ttyACM1",
                "/dev/ttyS0",
            ]
        };
        egui::ComboBox::from_id_salt("cat_serial_port_preset_combo")
            .width(85.0)
            .selected_text(if port_name.is_empty() {
                crate::core::i18n::tr_or(lang, "Wybierz...", "Select...")
            } else {
                port_name.as_str()
            })
            .show_ui(ui, |ui| {
                for &p in port_presets {
                    ui.selectable_value(port_name, p.to_string(), p);
                }
            });

        ui.add_space(6.0);
        ui.label(tr(key::CAT_SETTINGS_BAUD_RATE_LABEL, lang));
        egui::ComboBox::from_id_salt("cat_baud_combo")
            .selected_text(format!("{baud_rate} bps"))
            .show_ui(ui, |ui| {
                for &b in &[4800, 9600, 19200, 38400, 57600, 115_200] {
                    ui.selectable_value(baud_rate, b, format!("{b} bps"));
                }
            });
    });
}

fn render_rigctld_supervisor_controls(
    app: &mut SpLogApp,
    ui: &mut egui::Ui,
    lang: crate::core::i18n::Language,
) {
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("{}:", tr(key::CAT_SETTINGS_HAMLIB_SOURCE, lang))).strong(),
        );
        ui.selectable_value(
            &mut app.cat_hamlib_source,
            "bundled".to_string(),
            tr(key::CAT_SETTINGS_HAMLIB_SOURCE_BUNDLED, lang),
        );
        ui.selectable_value(
            &mut app.cat_hamlib_source,
            "system".to_string(),
            tr(key::CAT_SETTINGS_HAMLIB_SOURCE_SYSTEM, lang),
        );
    });

    let detected = crate::cat::supervisor::RigctldSupervisor::find_rigctld_binary(
        &app.cat_hamlib_source,
        if app.cat_custom_rigctld_path.is_empty() {
            None
        } else {
            Some(&app.cat_custom_rigctld_path)
        },
    );

    ui.horizontal(|ui| {
        if let Some(ref p) = detected {
            ui.colored_label(egui::Color32::from_rgb(34, 197, 94), "✔");
            ui.label(
                egui::RichText::new(format!(
                    "{}: {}",
                    tr(key::CAT_SETTINGS_DETECTED_BINARY, lang),
                    p.display()
                ))
                .size(11.0)
                .color(egui::Color32::from_rgb(148, 163, 184)),
            );
        } else {
            ui.colored_label(egui::Color32::from_rgb(239, 68, 68), "⚠");
            ui.label(
                egui::RichText::new(tr(key::CAT_SETTINGS_BINARY_NOT_FOUND, lang))
                    .size(11.0)
                    .color(egui::Color32::from_rgb(239, 68, 68)),
            );
        }
    });

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(tr(key::CAT_SETTINGS_CUSTOM_PATH_LABEL, lang)).size(11.0));
        ui.add(
            egui::TextEdit::singleline(&mut app.cat_custom_rigctld_path)
                .desired_width(220.0)
                .hint_text(tr(key::CAT_SETTINGS_CUSTOM_PATH_HINT, lang)),
        );
    });

    if let Some(ref mut sup) = app.rigctld_supervisor {
        if sup.is_running() {
            if let Some(pid) = sup.pid() {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::from_rgb(34, 197, 94), "●");
                    ui.label(
                        egui::RichText::new(format!(
                            "{} (PID: {pid})",
                            tr(key::CAT_SETTINGS_RUNNING_PID, lang)
                        ))
                        .size(11.0)
                        .color(egui::Color32::from_rgb(34, 197, 94)),
                    );
                });
            }
        }
    }
}

/// Renderuje asynchroniczny, strictly read-only probe runner.
fn render_probe_section(
    app: &mut SpLogApp,
    ui: &mut egui::Ui,
    rig: &RigConfig,
    lang: crate::core::i18n::Language,
) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            if ui.button(tr(key::CAT_SETTINGS_PROBE_BTN, lang)).clicked() && !app.cat_probe_running
            {
                if let Err(e) = rig.validate() {
                    app.cat_test_result = Some(format!(
                        "{}: {e}",
                        tr(key::CAT_SETTINGS_VALIDATION_FAILED, lang)
                    ));
                } else {
                    let (tx, rx) = std::sync::mpsc::channel();
                    app.cat_probe_rx = Some(rx);
                    app.cat_probe_running = true;
                    app.cat_probe_result = None;
                    let rig_copy = rig.clone();

                    tokio::spawn(async move {
                        let res = RigProbeRunner::probe(&rig_copy).await;
                        let _ = tx.send(res.map_err(|e| e.to_string()));
                    });
                }
            }

            if app.cat_probe_running {
                ui.spinner();
                ui.colored_label(
                    egui::Color32::from_rgb(250, 204, 21),
                    tr(key::CAT_SETTINGS_PROBE_RUNNING, lang),
                );
            }
        });

        if let Some(ref probe_res) = app.cat_probe_result {
            ui.add_space(2.0);
            match probe_res {
                Ok(res) => {
                    ui.colored_label(
                        egui::Color32::from_rgb(34, 197, 94),
                        format!("✔ {}", tr(key::CAT_SETTINGS_PROBE_SUCCESS, lang)),
                    );
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!(
                            "Identyfikacja: {}",
                            res.identity.as_deref().unwrap_or("—")
                        ));
                        ui.label("|");
                        ui.label(format!("Model: {}", res.model.as_deref().unwrap_or("—")));
                        ui.label("|");
                        ui.label(format!(
                            "{}: {}",
                            tr(key::CAT_SETTINGS_PROBE_FIRMWARE, lang),
                            res.firmware.as_deref().unwrap_or("—")
                        ));
                        if !res.safe_diagnostic.is_empty() {
                            ui.label("|");
                            ui.label(format!("Info: {}", res.safe_diagnostic));
                        }
                    });
                }
                Err(e) => {
                    ui.colored_label(
                        egui::Color32::from_rgb(239, 68, 68),
                        format!(
                            "✖ {}: {}",
                            tr(key::CAT_SETTINGS_PROBE_FAILED, lang),
                            redact_sensitive_info(e)
                        ),
                    );
                }
            }
        }

        if let Some(ref res) = app.cat_test_result {
            ui.add_space(2.0);
            ui.colored_label(egui::Color32::from_rgb(56, 189, 248), res);
        }
    });
}

/// Renderuje zestaw odznak możliwości technicznych oraz bezpieczną diagnostykę sesji.
fn render_capabilities_and_diagnostics(
    ui: &mut egui::Ui,
    svc_snap: Option<&crate::cat::service::RigServiceSnapshot>,
    probe_res: Option<&Result<crate::cat::backend::RigProbeResult, String>>,
    lang: crate::core::i18n::Language,
) {
    let caps = svc_snap
        .map(|s| s.capabilities)
        .or_else(|| {
            probe_res
                .and_then(|r| r.as_ref().ok())
                .map(|p| p.capabilities)
        })
        .unwrap_or_default();

    ui.group(|ui| {
        ui.label(
            egui::RichText::new(tr(key::CAT_SETTINGS_CAPABILITIES_TITLE, lang))
                .strong()
                .color(egui::Color32::from_rgb(56, 189, 248)),
        );

        ui.horizontal_wrapped(|ui| {
            render_cap_badge(ui, tr(key::CAT_SETTINGS_CAP_VFO, lang), caps.frequency);
            render_cap_badge(ui, tr(key::CAT_SETTINGS_CAP_MODE, lang), caps.mode);
            render_cap_badge(ui, tr(key::CAT_SETTINGS_CAP_PASSBAND, lang), caps.passband);
            render_cap_badge(ui, tr(key::CAT_SETTINGS_CAP_SPLIT, lang), caps.split);
            render_cap_badge(ui, tr(key::CAT_SETTINGS_CAP_PTT, lang), caps.ptt);
            render_cap_badge(
                ui,
                tr(key::CAT_SETTINGS_CAP_RIT, lang),
                caps.rit || caps.xit,
            );
            render_cap_badge(ui, tr(key::CAT_SETTINGS_CAP_POWER, lang), caps.power);
            render_cap_badge(ui, tr(key::CAT_SETTINGS_CAP_SMETER, lang), caps.s_meter);
        });

        if let Some(svc) = svc_snap {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(tr(key::CAT_SETTINGS_DIAGNOSTICS_TITLE, lang))
                    .size(11.0)
                    .color(egui::Color32::from_rgb(148, 163, 184)),
            );
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("Generacja stanu: {}", svc.state_generation));
                ui.label("|");
                ui.label(format!("Świeżość: {:?}", svc.freshness));
                if let Some(ref err) = svc.last_error {
                    ui.label("|");
                    ui.colored_label(
                        egui::Color32::from_rgb(239, 68, 68),
                        format!("Ostatni błąd: {}", redact_sensitive_info(err)),
                    );
                }
            });
        }
    });
}

fn render_cap_badge(ui: &mut egui::Ui, name: &str, active: bool) {
    let (bg, fg) = if active {
        (
            egui::Color32::from_rgb(22, 101, 52),
            egui::Color32::from_rgb(187, 247, 208),
        )
    } else {
        (
            egui::Color32::from_rgb(51, 65, 85),
            egui::Color32::from_rgb(148, 163, 184),
        )
    };

    ui.colored_label(
        bg,
        egui::RichText::new(format!(" {name} "))
            .color(fg)
            .size(11.0),
    );
}

fn render_cat_sharing_section(
    app: &mut SpLogApp,
    ui: &mut egui::Ui,
    lang: crate::core::i18n::Language,
) {
    ui.group(|ui| {
        ui.label(
            egui::RichText::new(tr(key::CAT_SETTINGS_SHARING_TITLE, lang))
                .strong()
                .color(egui::Color32::from_rgb(56, 189, 248)),
        );
        ui.horizontal(|ui| {
            if ui
                .checkbox(
                    &mut app.cat_sharing_enabled,
                    tr(key::CAT_SETTINGS_SHARING_ENABLE, lang),
                )
                .changed()
            {
                app.toggle_cat_proxy_server();
                app.save_station_config();
            }
            ui.add_space(10.0);
            ui.label(tr(key::CAT_SETTINGS_SERVER_PORT, lang));
            ui.add(egui::DragValue::new(&mut app.cat_sharing_port).range(1024..=65535));
        });

        if app.cat_sharing_active {
            ui.label(
                egui::RichText::new(format!("● 127.0.0.1:{}", app.cat_sharing_port))
                    .color(egui::Color32::from_rgb(34, 197, 94))
                    .size(11.0),
            );
        }
    });
}

fn render_rotor_section(app: &mut SpLogApp, ui: &mut egui::Ui, lang: crate::core::i18n::Language) {
    ui.group(|ui| {
        ui.label(
            egui::RichText::new(tr(key::CAT_SETTINGS_ROTOR_SECTION, lang))
                .strong()
                .color(egui::Color32::from_rgb(56, 189, 248)),
        );
        ui.horizontal(|ui| {
            ui.label("Host:");
            ui.add(egui::TextEdit::singleline(&mut app.rotor_host).desired_width(110.0));
            ui.add_space(10.0);
            ui.label("Port TCP:");
            let mut port_str = app.rotor_port.to_string();
            if ui
                .add(egui::TextEdit::singleline(&mut port_str).desired_width(60.0))
                .changed()
            {
                if let Ok(p) = port_str.trim().parse::<u16>() {
                    app.rotor_port = p;
                }
            }
        });

        if let Some(ref res) = app.rotor_test_result {
            ui.colored_label(egui::Color32::from_rgb(56, 189, 248), res);
        }
    });
}

fn sync_legacy_fields_from_rig(app: &mut SpLogApp, rig: &RigConfig) {
    app.cat_poll_rate_ms = rig.poll_interval_ms;
    app.cat_backend = rig.backend.kind_str().to_string();

    match &rig.backend {
        RigBackendConfig::Hamlib(cfg) => {
            app.cat_host = cfg.host.clone();
            app.cat_port = cfg.port;
        }
        RigBackendConfig::Flrig(cfg) => {
            app.cat_host = cfg.host.clone();
            app.cat_port = cfg.port;
        }
        RigBackendConfig::Tci(cfg) => {
            app.tci_host = cfg.host.clone();
            app.tci_port = cfg.port;
        }
        RigBackendConfig::Icom(cfg) => {
            app.cat_serial_port = cfg.port_name.clone();
            app.cat_baud_rate = cfg.baud_rate;
        }
        RigBackendConfig::Kenwood(cfg) => match &cfg.transport {
            KenwoodTransportConfig::Serial {
                port_name,
                baud_rate,
            } => {
                app.cat_serial_port = port_name.clone();
                app.cat_baud_rate = *baud_rate;
            }
            KenwoodTransportConfig::Lan { host, port } => {
                app.cat_host = host.clone();
                app.cat_port = *port;
            }
        },
        RigBackendConfig::Yaesu(cfg) => {
            app.cat_serial_port = cfg.port_name.clone();
            app.cat_baud_rate = cfg.baud_rate;
        }
        RigBackendConfig::Elecraft(cfg) => match &cfg.transport {
            ElecraftTransportConfig::Serial {
                port_name,
                baud_rate,
            } => {
                app.cat_serial_port = port_name.clone();
                app.cat_baud_rate = *baud_rate;
            }
            ElecraftTransportConfig::Tcp { host, port } => {
                app.cat_host = host.clone();
                app.cat_port = *port;
            }
        },
        RigBackendConfig::FlexRadio(cfg) => {
            if let FlexRadioConnectionConfig::Manual { host, port } = &cfg.connection {
                app.cat_host = host.clone();
                app.cat_port = *port;
            }
        }
    }
}
