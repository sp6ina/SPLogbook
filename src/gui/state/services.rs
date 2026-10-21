// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

//! Service handles and domain engines aggregated for the GUI.

use std::sync::{Arc, Mutex};

use crate::cloud::scheduler::UploadScheduler;
use crate::core::awards::AwardsEngine;
use crate::core::callbook::LocalCallbook;
use crate::core::database::LogDatabase;
use crate::core::events::EventBus;
use crate::core::ingest::QsoIngestor;
use crate::core::prefix::PrefixMatcher;
use crate::core::scp::ScpEngine;
use crate::core::service_db::ServiceDatabase;
use crate::plugins::PluginEngine;

/// Collection of service handles and domain engines used across the application.
pub struct AppServices {
    pub log_db: Arc<Mutex<LogDatabase>>,
    pub prefix_matcher: Arc<PrefixMatcher>,
    pub scp_engine: Arc<Mutex<ScpEngine>>,
    pub awards_engine: Arc<Mutex<AwardsEngine>>,
    pub service_db: Arc<ServiceDatabase>,
    pub local_callbook: Arc<LocalCallbook>,
    pub upload_scheduler: Arc<Mutex<UploadScheduler>>,
    pub event_bus: EventBus,
    pub plugin_engine: PluginEngine,
    pub ingestor: QsoIngestor,
}

impl AppServices {
    pub fn new(
        log_db: Arc<Mutex<LogDatabase>>,
        prefix_matcher: Arc<PrefixMatcher>,
        scp_engine: Arc<Mutex<ScpEngine>>,
        awards_engine: Arc<Mutex<AwardsEngine>>,
        service_db: Arc<ServiceDatabase>,
        local_callbook: Arc<LocalCallbook>,
        upload_scheduler: Arc<Mutex<UploadScheduler>>,
        event_bus: EventBus,
        plugin_engine: PluginEngine,
        ingestor: QsoIngestor,
    ) -> Self {
        Self {
            log_db,
            prefix_matcher,
            scp_engine,
            awards_engine,
            service_db,
            local_callbook,
            upload_scheduler,
            event_bus,
            plugin_engine,
            ingestor,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn services_construction() {
        let log_db = Arc::new(Mutex::new(LogDatabase::open_in_memory().unwrap()));
        let prefix_matcher = Arc::new(PrefixMatcher::empty());
        let scp_engine = Arc::new(Mutex::new(ScpEngine::new()));
        let awards_engine = Arc::new(Mutex::new(AwardsEngine::new()));
        let service_db = Arc::new(ServiceDatabase::open());
        let local_callbook = Arc::new(LocalCallbook::new(None, None));
        let upload_scheduler = Arc::new(Mutex::new(UploadScheduler::new()));
        let event_bus = EventBus::default();
        let plugin_engine = PluginEngine::new();
        let ingestor = QsoIngestor::new(
            log_db.clone(),
            awards_engine.clone(),
            scp_engine.clone(),
            prefix_matcher.clone(),
            event_bus.clone(),
        );

        let services = AppServices::new(
            log_db,
            prefix_matcher,
            scp_engine,
            awards_engine,
            service_db,
            local_callbook,
            upload_scheduler,
            event_bus,
            plugin_engine,
            ingestor,
        );

        assert!(services.log_db.lock().is_ok());
    }
}
