// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Application runtime state and lifecycle policy.

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use crate::cluster::telnet::DxSpot;

/// Policy controlling shutdown actions (such as automatic database backup on Drop).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ShutdownPolicy {
    #[default]
    None,
    Backup,
}

/// Runtime flags, server handles, and polling timestamps.
pub struct AppRuntime {
    pub shutdown_policy: ShutdownPolicy,
    pub initial_cat_enabled: bool,
    pub rest_api_enabled: bool,
    pub rest_api_port: u16,
    pub api_reload_flag: Arc<AtomicBool>,
    pub cluster_spots_api: Option<Arc<Mutex<Vec<DxSpot>>>>,
    pub last_upload_poll_secs: u64,
    pub qrz_lookup_tx: std::sync::mpsc::Sender<crate::cloud::qrz::CallbookData>,
    pub qrz_lookup_rx: std::sync::mpsc::Receiver<crate::cloud::qrz::CallbookData>,
}

impl Default for AppRuntime {
    fn default() -> Self {
        Self::new(ShutdownPolicy::None, false)
    }
}

impl AppRuntime {
    pub fn new(shutdown_policy: ShutdownPolicy, initial_cat_enabled: bool) -> Self {
        let (qrz_lookup_tx, qrz_lookup_rx) = std::sync::mpsc::channel();
        Self {
            shutdown_policy,
            initial_cat_enabled,
            rest_api_enabled: false,
            rest_api_port: 8080,
            api_reload_flag: Arc::new(AtomicBool::new(false)),
            cluster_spots_api: None,
            last_upload_poll_secs: 0,
            qrz_lookup_tx,
            qrz_lookup_rx,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn default_matches_initial_values() {
        let rt = AppRuntime::default();
        assert_eq!(rt.shutdown_policy, ShutdownPolicy::None);
        assert!(!rt.initial_cat_enabled);
        assert!(!rt.rest_api_enabled);
        assert_eq!(rt.rest_api_port, 8080);
        assert!(!rt.api_reload_flag.load(Ordering::Relaxed));
        assert!(rt.cluster_spots_api.is_none());
        assert_eq!(rt.last_upload_poll_secs, 0);
        assert!(rt.qrz_lookup_rx.try_recv().is_err());
    }
}
