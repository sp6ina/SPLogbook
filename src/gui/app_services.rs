use super::SpLogApp;
use crate::core::i18n::{Msg, key, tr};
use std::sync::{Arc, Mutex};

impl SpLogApp {
    /// Zwraca istniejący klucz uwierzytelniający REST API lub generuje nowy (i zapisuje
    /// konfigurację), jeśli jeszcze nie istnieje. Klucz jest wymagany przez klientów
    /// w nagłówku `X-Api-Key`, aby zapobiec dostępowi z dowolnej strony/skryptu
    /// działającego lokalnie (serwer nasłuchuje na 127.0.0.1, ale CORS jest otwarty).
    pub fn ensure_rest_api_key(&mut self) -> String {
        if self.rest_api_key.is_empty() {
            self.rest_api_key = crate::api::server::generate_api_key();
            self.save_station_config();
        }
        self.rest_api_key.clone()
    }

    /// Uruchamia wbudowany serwer REST API w osobnym zadaniu tokio, wykorzystując
    /// bieżący stan aplikacji (baza, znak wywoławczy, port, klucz API, spoty klastra,
    /// prefiksy DXCC, stan radia CAT i silnik dyplomowy).
    pub fn start_rest_api_server(&mut self) {
        let db = self.services.log_db.clone();
        let cs = self.my_station.callsign.clone();
        let port = self.rest_api_port;
        let key = self.ensure_rest_api_key();
        let spots_for_api = Arc::new(Mutex::new(self.cluster_spots.clone()));
        self.cluster_spots_api = Some(spots_for_api.clone());
        if let Ok(mut shared_rig) = self.cat_shared_state.write() {
            *shared_rig = self.rig_state.clone();
        }
        let state = crate::api::server::ApiState::new(
            db,
            cs,
            spots_for_api,
            key,
            self.services.event_bus.clone(),
        )
        .with_station_context(
            self.services.prefix_matcher.clone(),
            self.cat_shared_state.clone(),
            self.services.awards_engine.clone(),
            self.api_reload_flag.clone(),
        )
        .with_ingestor(self.services.ingestor.clone());
        tokio::spawn(async move {
            crate::api::server::start_api_server_with_state(state, port).await;
        });
    }

    pub fn connect_dx_cluster(&mut self) {
        self.disconnect_dx_cluster();

        if crate::cluster::telnet::is_dead_legacy_cluster_host(&self.cluster_host) {
            self.cluster_host = "dxcluster.pl".to_string();
            self.cluster_port = 8000;
        }

        let host = self.cluster_host.clone();
        let port = self.cluster_port;
        let call = if self.cluster_callsign.trim().is_empty() {
            self.my_station.callsign.clone()
        } else {
            self.cluster_callsign.trim().to_uppercase()
        };

        let (event_tx, event_rx) = std::sync::mpsc::channel();
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
        let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel();

        self.cluster_event_rx = Some(event_rx);
        self.cluster_stop_tx = Some(stop_tx);
        self.cluster_cmd_tx = Some(cmd_tx);
        self.cluster_connecting = true;
        self.cluster_connected = false;
        self.cluster_status_text = Msg::new(key::CLUSTER_CONNECTING_TO)
            .arg("host", &host)
            .arg("port", port)
            .text(self.gui.appearance.language);

        tokio::spawn(async move {
            crate::cluster::telnet::DxClusterClient::run_with_events_and_commands(
                host,
                port,
                call,
                event_tx,
                stop_rx,
                Some(cmd_rx),
            )
            .await;
        });
    }

    pub fn disconnect_dx_cluster(&mut self) {
        if let Some(stop_tx) = self.cluster_stop_tx.take() {
            let _ = stop_tx.send(true);
        }
        self.cluster_event_rx = None;
        self.cluster_cmd_tx = None;
        self.cluster_connected = false;
        self.cluster_connecting = false;
        self.cluster_status_text =
            tr(key::CLUSTER_DISCONNECTED, self.gui.appearance.language).to_string();
    }

    pub fn trigger_database_update(&mut self) {
        let lang = self.gui.appearance.language;
        let db_dir = if std::path::Path::new("databases").is_dir() {
            std::path::PathBuf::from("databases")
        } else {
            self.paths
                .active_db_path
                .parent()
                .map(|p| p.join("databases"))
                .unwrap_or_else(|| std::path::PathBuf::from("databases"))
        };
        let tx = self.sync_log_tx.clone();
        let scp = self.services.scp_engine.clone();

        tokio::spawn(async move {
            let outcome =
                crate::cloud::updater::DatabaseUpdater::update_all_with_report(&db_dir, lang).await;

            // Przeładuj świeżo pobrany plik MASTER.SCP do pamięci silnika SCP
            let scp_path = db_dir.join("MASTER.SCP");
            if let Ok(file) = std::fs::File::open(&scp_path) {
                let reader = std::io::BufReader::new(file);
                if let Ok(mut guard) = scp.lock() {
                    guard.load_from_reader(reader);
                }
            }

            match outcome {
                Ok(msg) => {
                    let _ = tx.send((msg.to_string(), true));
                }
                Err(err_msg) => {
                    let _ = tx.send((err_msg.to_string(), false));
                }
            }
        });

        let start_msg = tr(key::UPDATER_DOWNLOADING_DATABASES, lang).to_string();
        self.gui.chrome.status_message = Some(start_msg.clone());
        self.gui.chrome.status_toast = Some((start_msg, std::time::Instant::now()));
    }

    /// Sprawdza najnowsze wydanie na GitHub w tle i zapisuje wynik do odbiornika.
    pub fn trigger_update_check(&mut self) {
        self.updates.update_check_status =
            Some("Sprawdzanie najnowszej wersji na GitHub…".to_string());
        self.updates.update_available = None;
        let (tx, rx) = std::sync::mpsc::channel::<crate::cloud::updater::UpdateCheckOutcome>();
        self.updates.update_check_rx = Some(rx);
        tokio::spawn(async move {
            let outcome = match crate::cloud::updater::latest_release().await {
                Ok(release) => {
                    let local = env!("CARGO_PKG_VERSION");
                    if release.tag == local {
                        crate::cloud::updater::UpdateCheckOutcome::UpToDate {
                            local: local.to_string(),
                        }
                    } else {
                        crate::cloud::updater::UpdateCheckOutcome::NewVersion(release)
                    }
                }
                Err(e) => crate::cloud::updater::UpdateCheckOutcome::Error(e.to_string()),
            };
            let _ = tx.send(outcome);
        });
    }

    /// Pobiera i instaluje dostępną aktualizację w tle (self-replace + restart).
    pub fn trigger_update_install(&mut self) {
        let Some(release) = self.updates.update_available.clone() else {
            return;
        };
        let Some(asset) =
            crate::cloud::updater::select_asset_for_platform(&release.assets).cloned()
        else {
            self.updates.update_install_status =
                Some("❌ Wydanie nie zawiera pliku instalacyjnego dla tego systemu.".to_string());
            return;
        };

        self.updates.update_install_status = Some(format!(
            "Pobieranie aktualizacji do v{} ({}: {})…",
            release.tag,
            asset.name,
            if asset.size > 0 {
                format!("{:.1} MB", asset.size as f64 / 1_048_576.0)
            } else {
                "?".to_string()
            }
        ));

        let (tx, rx) = std::sync::mpsc::channel::<String>();
        self.updates.update_install_rx = Some(rx);
        let lang = self.gui.appearance.language;
        tokio::spawn(async move {
            let msg = match crate::cloud::updater::install_update(&asset).await {
                Ok(()) => tr(key::UPDATER_DOWNLOADED_RESTART, lang).to_string(),
                Err(e) => Msg::new(key::UPDATER_INSTALL_FAILED)
                    .arg("err", &e)
                    .text(lang),
            };
            let _ = tx.send(msg);
        });
    }

    /// Obsługuje wspólną kolejkę wysyłki: pobiera gotowe zadania i uruchamia
    /// ich wysyłkę w tle, a po zakończeniu zapisuje wynik z powrotem do kolejki.
    pub fn poll_upload_scheduler(&mut self) {
        use crate::cloud::scheduler::UploadService;

        let now = crate::cloud::scheduler::now_unix();
        // Ogranicz sprawdzanie kolejki do maksymalnie raz na 5 sekund (redukcja rywalizacji o Mutex w pętli 60Hz UI)
        if now < self.last_upload_poll_secs + 5 {
            return;
        }
        self.last_upload_poll_secs = now;

        let creds = crate::cloud::scheduler::UploadCredentials {
            clublog_callsign: self.online.clublog_callsign.clone(),
            clublog_email: self.online.clublog_email.clone(),
            clublog_password: self.online.clublog_password.clone(),
            clublog_api_key: self.online.clublog_api_key.clone(),
            qrz_api_key: self.qrz_api_key.clone(),
            eqsl_username: self.online.eqsl_username.clone(),
            eqsl_password: self.online.eqsl_password.clone(),
        };

        // Pobierz gotowe zadania (maks. jedno na serwis, z uwzględnieniem rate-limit).
        let ready: Vec<(u64, UploadService, String)> = {
            let mut sched = self
                .services
                .upload_scheduler
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let ids = sched.ready_jobs(now);
            ids.into_iter()
                .filter_map(|id| {
                    sched.mark_in_flight(id, now);
                    let job = sched.job(id)?;
                    Some((id, job.service, job.adif.clone()))
                })
                .collect()
        };

        if ready.is_empty() {
            return;
        }

        let sched = self.services.upload_scheduler.clone();
        tokio::spawn(async move {
            for (id, service, adif) in ready {
                let result = crate::cloud::scheduler::execute_upload(service, &creds, &adif).await;
                let now = crate::cloud::scheduler::now_unix();
                let mut sched = sched
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                match result {
                    Ok(_) => sched.mark_success(id, now),
                    Err(e) => {
                        sched.mark_failure(id, e.to_string(), now);
                    }
                }
                sched.save_to_disk();
            }
            // Po zakończeniu serii utrzymaj kolejkę w rozsądnym rozmiarze.
            let mut sched = sched
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if sched.purge_done() > 0 {
                sched.save_to_disk();
            }
        });
    }
}
