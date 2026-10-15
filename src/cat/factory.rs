// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)
//! Fabryka backendów radiowych ([`RigBackendFactory`]) (NATIVE-6B).
//!
//! Tworzy instancje [`RigBackend`] dla wszystkich 8 wspieranych backendów
//! na podstawie zwalidowanego modelu [`RigConfig`].
//!
//! # Gwarancje architektoniczne:
//! - Dokładnie jeden wariant konfiguracji na backend.
//! - Brak stringowego dispatchu w logice tworzenia.
//! - Całkowity zakaz fallbacku na Hamlib lub inny pierwszy backend.
//! - Tworzenie backendu nie wykonuje I/O, nie nawiązuje połączenia i nie keyuje nadajnika.
//! - Bezpieczny `Debug` nieujawniający danych poufnych.

use std::fmt;

use crate::cat::backend::RigBackend;
use crate::cat::config::{
    CatConfigValidationError, ElecraftTransportConfig, FlexRadioConnectionConfig,
    KenwoodTransportConfig, RigBackendConfig, RigConfig,
};
use crate::cat::flrig::FlrigBackend;
use crate::cat::hamlib::HamlibBackend;
use crate::cat::native::elecraft::models::{ELECRAFT_K3_PROFILE, find_elecraft_profile};
use crate::cat::native::elecraft::{ElecraftBackend, ElecraftConnection, ElecraftTcpConfig};
use crate::cat::native::flexradio::FlexRadioBackend;
use crate::cat::native::flexradio::client::FlexRadioConfig;
use crate::cat::native::flexradio::slice::FlexSliceId;
use crate::cat::native::icom::IcomCiVBackend;
use crate::cat::native::icom::models::create_custom_icom_profile;
use crate::cat::native::kenwood::KenwoodBackend;
use crate::cat::native::kenwood::lan::KenwoodLanConfig;
use crate::cat::native::kenwood::models::create_custom_kenwood_profile;
use crate::cat::native::registry::find_profile_by_model;
use crate::cat::native::transport::SerialConfig;
use crate::cat::native::yaesu::YaesuBackend;
use crate::cat::native::yaesu::models::create_custom_yaesu_profile;
use crate::cat::tci::TciBackend;

/// Typowane błędy tworzenia backendu radiowego w fabryce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactoryError {
    /// Błąd walidacji konfiguracji.
    InvalidConfig(CatConfigValidationError),
    /// Wskazany model transceivera nie został odnaleziony w rejestrze profili.
    ProfileNotFound {
        /// Nazwa poszukiwanego modelu.
        model: String,
    },
    /// Backend nie jest obsługiwany przez fabrykę.
    UnsupportedBackend(String),
}

impl fmt::Display for FactoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(e) => write!(f, "błąd konfiguracji backendu: {e}"),
            Self::ProfileNotFound { model } => {
                write!(f, "nie odnaleziono profilu modelu radia: '{model}'")
            }
            Self::UnsupportedBackend(b) => write!(f, "nieobsługiwany backend CAT: '{b}'"),
        }
    }
}

impl std::error::Error for FactoryError {}

impl From<CatConfigValidationError> for FactoryError {
    fn from(e: CatConfigValidationError) -> Self {
        Self::InvalidConfig(e)
    }
}

/// Kanoniczna fabryka instancji [`RigBackend`].
#[derive(Debug, Default, Clone, Copy)]
pub struct RigBackendFactory;

impl RigBackendFactory {
    /// Tworzy nowy backend radiowy na podstawie konfiguracji [`RigConfig`].
    ///
    /// # Gwarancje:
    /// - Brak fallbacku na Hamlib przy nieznanych danych.
    /// - Fabryka nie łączy się ze sprzętem ani demonem.
    /// - Zwraca typed error w razie nieprawidłowej konfiguracji.
    pub fn create(config: &RigConfig) -> Result<Box<dyn RigBackend>, FactoryError> {
        config.validate()?;

        match &config.backend {
            RigBackendConfig::Hamlib(cfg) => {
                let backend = HamlibBackend::new(&cfg.host, cfg.port);
                Ok(Box::new(backend))
            }
            RigBackendConfig::Flrig(cfg) => {
                let backend = FlrigBackend::new(&cfg.host, cfg.port);
                Ok(Box::new(backend))
            }
            RigBackendConfig::Tci(cfg) => {
                let backend = TciBackend::new(&cfg.host, cfg.port);
                Ok(Box::new(backend))
            }
            RigBackendConfig::Icom(cfg) => {
                let profile = if !cfg.model.trim().is_empty() {
                    find_profile_by_model(&cfg.model)
                        .cloned()
                        .unwrap_or_else(|| {
                            create_custom_icom_profile(&cfg.model, cfg.civ_address, cfg.baud_rate)
                        })
                } else {
                    create_custom_icom_profile("IC-Custom", cfg.civ_address, cfg.baud_rate)
                };

                let serial_cfg = SerialConfig::new(&cfg.port_name, cfg.baud_rate);
                let backend = IcomCiVBackend::new(serial_cfg, profile, Some(cfg.civ_address));
                Ok(Box::new(backend))
            }
            RigBackendConfig::Kenwood(cfg) => {
                let profile = if !cfg.model.trim().is_empty() {
                    find_profile_by_model(&cfg.model)
                        .cloned()
                        .unwrap_or_else(|| create_custom_kenwood_profile(&cfg.model, 115200))
                } else {
                    create_custom_kenwood_profile("TS-Custom", 115200)
                };

                match &cfg.transport {
                    KenwoodTransportConfig::Serial {
                        port_name,
                        baud_rate,
                    } => {
                        let serial_cfg = SerialConfig::new(port_name, *baud_rate);
                        let backend = KenwoodBackend::new_serial(serial_cfg, profile);
                        Ok(Box::new(backend))
                    }
                    KenwoodTransportConfig::Lan { host, port } => {
                        let mut lan_cfg = KenwoodLanConfig::new(host);
                        lan_cfg.port = *port;
                        let backend = KenwoodBackend::new_lan(lan_cfg, profile);
                        Ok(Box::new(backend))
                    }
                }
            }
            RigBackendConfig::Yaesu(cfg) => {
                let profile = if !cfg.model.trim().is_empty() {
                    find_profile_by_model(&cfg.model)
                        .cloned()
                        .unwrap_or_else(|| create_custom_yaesu_profile(&cfg.model, cfg.baud_rate))
                } else {
                    create_custom_yaesu_profile("FT-Custom", cfg.baud_rate)
                };

                let serial_cfg = SerialConfig::new(&cfg.port_name, cfg.baud_rate);
                let backend = YaesuBackend::new_serial(serial_cfg, profile);
                Ok(Box::new(backend))
            }
            RigBackendConfig::Elecraft(cfg) => {
                let profile = if !cfg.model.trim().is_empty() {
                    find_elecraft_profile(&cfg.model)
                        .cloned()
                        .unwrap_or_else(|| ELECRAFT_K3_PROFILE.clone())
                } else {
                    ELECRAFT_K3_PROFILE.clone()
                };

                match &cfg.transport {
                    ElecraftTransportConfig::Serial {
                        port_name,
                        baud_rate,
                    } => {
                        let serial_cfg = SerialConfig::new(port_name, *baud_rate);
                        let conn = ElecraftConnection::Serial(serial_cfg);
                        let backend = ElecraftBackend::new(conn, profile);
                        Ok(Box::new(backend))
                    }
                    ElecraftTransportConfig::Tcp { host, port } => {
                        let mut tcp_cfg = ElecraftTcpConfig::default();
                        tcp_cfg.host = host.clone();
                        tcp_cfg.port = *port;
                        let conn = ElecraftConnection::Tcp(tcp_cfg);
                        let backend = ElecraftBackend::new(conn, profile);
                        Ok(Box::new(backend))
                    }
                }
            }
            RigBackendConfig::FlexRadio(cfg) => {
                let mut flex_config = match &cfg.connection {
                    FlexRadioConnectionConfig::AutoDiscovery => FlexRadioConfig::default(),
                    FlexRadioConnectionConfig::Manual { host, port } => {
                        let mut c = FlexRadioConfig::default();
                        c.host = host.clone();
                        c.port = *port;
                        c
                    }
                };

                if let Some(slice_idx) = cfg.slice_id {
                    flex_config.preferred_slice = Some(FlexSliceId(slice_idx));
                }

                let backend = FlexRadioBackend::new(flex_config, cfg.model.as_deref());
                Ok(Box::new(backend))
            }
        }
    }
}
