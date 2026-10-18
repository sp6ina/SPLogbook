// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Mariusz Woźniak (SP6INA)

use axum::{
    Json, Router,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{Path, Query, State},
    http::{HeaderValue, Method, StatusCode, header},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use crate::cat::hamlib::RigState;
use crate::cluster::telnet::DxSpot;
use crate::core::adif::{self, ADIF_VERSION};
use crate::core::awards::{AwardsEngine, QsoAwardStatus};
use crate::core::clubs::{ClubAffiliation, ClubRegistry};
use crate::core::database::{AdvancedQsoFilter, Journal, LogDatabase};
use crate::core::events::{AppEvent, EventBus};
use crate::core::i18n::Language;
use crate::core::ingest::{ImportOptions, IngestError, QsoIngestor, QsoOrigin};
use crate::core::prefix::PrefixMatcher;
use crate::core::qso::QsoRecord;
use crate::core::scp::ScpEngine;

/// Ogranicza opcjonalną wartość `limit`/`offset` do przedziału `[0, max]`,
/// aby zapytanie typu `limit=1_000_000` nie wymuszało nadmiernego transferu.
fn clamp_usize(value: Option<usize>, default: usize, max: usize) -> usize {
    value.unwrap_or(default).min(max)
}

const MAX_QSO_LIMIT: usize = 1000;
const MAX_QSO_OFFSET: usize = 500_000;
const MAX_SPOT_LIMIT: usize = 200;

#[derive(Clone)]
pub struct ApiState {
    pub db: Arc<Mutex<LogDatabase>>,
    pub callsign: String,
    pub start_time: Instant,
    pub cluster_spots: Arc<Mutex<Vec<DxSpot>>>,
    /// Centralna magistrala zdarzeń do przesyłania na żywo przez WebSocket.
    pub events: EventBus,
    pub api_key: String,
    pub prefix_matcher: Arc<PrefixMatcher>,
    pub rig_state: Arc<RwLock<RigState>>,
    pub awards_engine: Arc<Mutex<AwardsEngine>>,
    pub scp_engine: Arc<Mutex<ScpEngine>>,
    /// Flaga informująca główną pętlę GUI o zmianie zawartości bazy przez REST API.
    pub reload_flag: Arc<AtomicBool>,
    pub ingestor: QsoIngestor,
}

impl ApiState {
    pub fn new(
        db: Arc<Mutex<LogDatabase>>,
        callsign: String,
        cluster_spots: Arc<Mutex<Vec<DxSpot>>>,
        api_key: String,
        events: EventBus,
    ) -> Self {
        let awards_engine = Arc::new(Mutex::new(AwardsEngine::new()));
        let prefix_matcher = Arc::new(PrefixMatcher::empty());
        let scp_engine = Arc::new(Mutex::new(ScpEngine::new()));
        let ingestor = QsoIngestor::new(
            Arc::clone(&db),
            Arc::clone(&awards_engine),
            Arc::clone(&scp_engine),
            Arc::clone(&prefix_matcher),
            events.clone(),
        );
        Self {
            db,
            callsign,
            start_time: Instant::now(),
            cluster_spots,
            events,
            api_key,
            prefix_matcher,
            rig_state: Arc::new(RwLock::new(RigState::default())),
            awards_engine,
            scp_engine,
            reload_flag: Arc::new(AtomicBool::new(false)),
            ingestor,
        }
    }

    pub fn with_station_context(
        mut self,
        prefix_matcher: Arc<PrefixMatcher>,
        rig_state: Arc<RwLock<RigState>>,
        awards_engine: Arc<Mutex<AwardsEngine>>,
        reload_flag: Arc<AtomicBool>,
    ) -> Self {
        self.prefix_matcher = prefix_matcher;
        self.rig_state = rig_state;
        self.awards_engine = awards_engine;
        self.reload_flag = reload_flag;
        self.ingestor = QsoIngestor::new(
            Arc::clone(&self.db),
            Arc::clone(&self.awards_engine),
            Arc::clone(&self.scp_engine),
            Arc::clone(&self.prefix_matcher),
            self.events.clone(),
        );
        self
    }

    pub fn with_ingestor(mut self, ingestor: QsoIngestor) -> Self {
        self.ingestor = ingestor;
        self
    }
}

/// Generuje losowy klucz uwierzytelniający dla lokalnego serwera REST API
/// za pomocą kryptograficznego generatora losowego (`rand::fill`).
pub fn generate_api_key() -> String {
    use std::fmt::Write as _;
    let mut bytes = [0u8; 32];
    rand::fill(&mut bytes);
    let mut key = String::with_capacity(64);
    for byte in bytes {
        let _ = write!(key, "{byte:02x}");
    }
    key
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub version: String,
    pub adif_version: String,
    pub callsign: String,
    pub total_qsos: usize,
    pub uptime_secs: u64,
    pub rig_connected: bool,
    pub frequency_hz: u64,
    pub mode: String,
}

#[derive(Serialize)]
pub struct EndpointDoc {
    pub method: &'static str,
    pub path: &'static str,
    pub auth_required: bool,
    pub description: &'static str,
}

#[derive(Deserialize, Default)]
pub struct QsoQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub callsign: Option<String>,
    pub band: Option<String>,
    pub mode: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub journal_id: Option<String>,
    pub lotw_confirmed: Option<bool>,
    pub eqsl_confirmed: Option<bool>,
    pub qsl_rcvd: Option<bool>,
}

#[derive(Deserialize, Default)]
pub struct DupeCheckQuery {
    pub band: Option<String>,
    pub mode: Option<String>,
    pub same_day: Option<bool>,
}

#[derive(Serialize)]
pub struct CallsignHistoryResponse {
    pub callsign: String,
    pub worked_before: bool,
    pub total_qsos: usize,
    pub is_dupe: bool,
    pub previous_qsos: Vec<QsoRecord>,
}

#[derive(Deserialize, Default)]
pub struct LookupQuery {
    pub band: Option<String>,
    pub mode: Option<String>,
}

#[derive(Serialize)]
pub struct LookupResponse {
    pub callsign: String,
    pub wpx_prefix: String,
    pub dxcc: Option<u32>,
    pub country: Option<String>,
    pub continent: Option<String>,
    pub cqz: Option<u32>,
    pub ituz: Option<u32>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub polish_district: Option<String>,
    pub clubs: Vec<ClubAffiliation>,
    pub award_status: Option<QsoAwardStatus>,
    pub previous_qso_count: usize,
}

#[derive(Serialize)]
pub struct StatsResponse {
    pub total: usize,
    pub unique_calls: i64,
    pub unique_dxcc: i64,
    pub by_band: HashMap<String, i64>,
    pub by_mode: HashMap<String, i64>,
    pub by_continent: HashMap<String, i64>,
    pub qsl_lotw: i64,
    pub qsl_eqsl: i64,
    pub qsl_paper: i64,
}

#[derive(Serialize)]
pub struct AwardsSummaryResponse {
    pub dxcc_worked: usize,
    pub dxcc_confirmed: usize,
    pub waz_worked: usize,
    pub waz_confirmed: usize,
    pub wac_worked: usize,
    pub wac_confirmed: usize,
    pub was_worked: usize,
    pub was_confirmed: usize,
    pub wpx_worked: usize,
    pub vucc_grids_worked: usize,
    pub iota_worked: usize,
    pub sota_worked: usize,
    pub pota_worked: usize,
    pub pga_worked: usize,
    pub sp_districts_worked: usize,
    pub sp_districts_confirmed: usize,
}

#[derive(Serialize)]
pub struct RigStateResponse {
    pub connected: bool,
    pub frequency_hz: u64,
    pub frequency_mhz: f64,
    pub band: String,
    pub mode: String,
    pub passband_hz: u32,
    pub vfo: String,
    pub split_enabled: bool,
    pub s_meter_dbm: f32,
    pub s_meter_unit: String,
    pub rf_power_watts: f32,
    pub ptt: bool,
}

#[derive(Deserialize)]
pub struct RigControlRequest {
    pub frequency_hz: Option<u64>,
    pub mode: Option<String>,
    pub passband_hz: Option<u32>,
    pub vfo: Option<String>,
    pub split_enabled: Option<bool>,
    pub ptt: Option<bool>,
}

#[derive(Serialize)]
pub struct AdifImportApiResponse {
    pub imported: usize,
    pub rejected: usize,
    pub errors: Vec<String>,
    pub total_qsos: usize,
}

#[derive(Serialize)]
pub struct JournalsResponse {
    pub active_journal_id: String,
    pub journals: Vec<Journal>,
}

#[derive(Deserialize, Default)]
pub struct ClusterQuery {
    pub limit: Option<usize>,
    pub band: Option<String>,
    pub ft8: Option<bool>,
    pub call: Option<String>,
}

/// Akceptuje wyłącznie originy loopback (localhost / 127.0.0.1 / [::1]), dzięki
/// czemu nagłówek CORS nie jest wystawiany dla dowolnej domeny (`*`), co ogranicza
/// powierzchnię ataku typu „localhost drive-by" / DNS rebinding.
fn loopback_origin_header(origin: &HeaderValue) -> Option<HeaderValue> {
    let origin_str = origin.to_str().ok()?;
    let rest = origin_str
        .strip_prefix("http://")
        .or_else(|| origin_str.strip_prefix("https://"))?;
    let authority = rest.split('/').next()?;
    let host = if let Some(bracketed) = authority.strip_prefix('[') {
        bracketed.split(']').next()?
    } else {
        authority.split(':').next()?
    };
    match host {
        "localhost" | "127.0.0.1" | "::1" => Some(origin.clone()),
        _ => None,
    }
}

async fn cors_middleware(req: axum::extract::Request, next: axum::middleware::Next) -> Response {
    const METHODS: HeaderValue = HeaderValue::from_static("GET, POST, PUT, DELETE, OPTIONS");
    const HEADERS: HeaderValue = HeaderValue::from_static("Content-Type, Authorization, X-Api-Key");
    const VARY_ORIGIN: HeaderValue = HeaderValue::from_static("Origin");

    let allow_origin = req
        .headers()
        .get(header::ORIGIN)
        .and_then(loopback_origin_header);

    if req.method() == Method::OPTIONS {
        let mut res = Response::new(axum::body::Body::empty());
        if let Some(origin) = allow_origin {
            res.headers_mut()
                .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
            res.headers_mut()
                .insert(header::ACCESS_CONTROL_ALLOW_METHODS, METHODS);
            res.headers_mut()
                .insert(header::ACCESS_CONTROL_ALLOW_HEADERS, HEADERS);
            res.headers_mut().insert(header::VARY, VARY_ORIGIN);
        }
        return res;
    }

    let mut res = next.run(req).await;
    if let Some(origin) = allow_origin {
        res.headers_mut()
            .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
        res.headers_mut().insert(header::VARY, VARY_ORIGIN);
    }
    res
}

/// Porównuje dwa ciągi bajtów w stałym czasie (ochrona przed atakami czasowymi).
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Wymaga poprawnego nagłówka `X-Api-Key` dla wszystkich żądań poza publicznymi
/// `/api/v1/status`, `/api/v1/endpoints` i preflightem CORS (OPTIONS).
/// Dla połączeń WebSocket (`/api/v1/ws`), gdy nagłówek `X-Api-Key` jest nieobecny,
/// dopuszcza odczyt klucza z parametru zapytania `?api_key=...`.
/// Zapobiega to odczytowi/zapisowi dziennika przez dowolną stronę WWW lub proces
/// lokalny, który mógłby wykorzystać otwarte CORS (`*`) do wysyłania żądań do
/// localhost (atak typu "localhost drive-by" / DNS rebinding).
async fn auth_middleware(
    State(state): State<ApiState>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    if req.method() == Method::OPTIONS || path == "/api/v1/status" || path == "/api/v1/endpoints" {
        return Ok(next.run(req).await);
    }

    let header_key = req.headers().get("x-api-key").and_then(|v| v.to_str().ok());
    let query_key = if header_key.is_none() && path == "/api/v1/ws" {
        req.uri().query().and_then(|q| {
            q.split('&').find_map(|pair| {
                let (k, v) = pair.split_once('=')?;
                if k == "api_key" { Some(v) } else { None }
            })
        })
    } else {
        None
    };

    let provided = header_key.or(query_key);
    match provided {
        Some(key)
            if !state.api_key.is_empty()
                && constant_time_eq(key.as_bytes(), state.api_key.as_bytes()) =>
        {
            Ok(next.run(req).await)
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

/// Buduje kompletny router Axum dla REST & WebSocket API v1.
pub fn build_api_router(state: ApiState) -> Router {
    Router::new()
        .route("/api/v1/status", get(get_status))
        .route("/api/v1/endpoints", get(get_endpoints))
        .route("/api/v1/qsos", get(get_qsos).post(post_qso))
        .route("/api/v1/qsos/callsign/{call}", get(get_qsos_by_callsign))
        .route(
            "/api/v1/qsos/{id}",
            get(get_qso_by_id)
                .put(put_qso_by_id)
                .delete(delete_qso_by_id),
        )
        .route("/api/v1/lookup/{call}", get(lookup_callsign))
        .route("/api/v1/adif/export", get(export_adif_handler))
        .route("/api/v1/adif/import", post(import_adif_handler))
        .route("/api/v1/rig", get(get_rig_state).post(post_rig_control))
        .route("/api/v1/awards", get(get_awards_summary))
        .route("/api/v1/journals", get(get_journals))
        .route("/api/v1/stats", get(get_stats))
        .route("/api/v1/cluster/spots", get(get_cluster_spots))
        .route("/api/v1/ws", get(ws_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .layer(middleware::from_fn(cors_middleware))
        .with_state(state)
}

pub async fn start_api_server(
    db: Arc<Mutex<LogDatabase>>,
    callsign: String,
    port: u16,
    cluster_spots: Arc<Mutex<Vec<DxSpot>>>,
    api_key: String,
    events: EventBus,
) {
    let state = ApiState::new(db, callsign, cluster_spots, api_key, events);
    start_api_server_with_state(state, port).await;
}

pub async fn start_api_server_with_state(state: ApiState, port: u16) {
    let app = build_api_router(state);

    let bind_addr = format!("127.0.0.1:{port}");
    let listener = match tokio::net::TcpListener::bind(&bind_addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!(
                "[REST API] Nie można uruchomić serwera na {bind_addr}: {e}. \
                Zmień port w menu Narzędzia -> REST API lub zwolnij port."
            );
            return;
        }
    };
    eprintln!("[REST API] Serwer uruchomiony na http://{bind_addr}");
    eprintln!(
        "[REST API] Wymagany nagłówek uwierzytelniający X-Api-Key (patrz Narzędzia -> REST API)."
    );
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("[REST API] Błąd serwera: {e}");
    }
}

/// Wykonuje operację na bazie danych poza wątkami roboczymi Tokio.
/// Długie zapytania SQLite nie blokują wtedy obsługi pozostałych endpointów.
async fn with_db<T, F>(state: &ApiState, f: F) -> Result<T, (StatusCode, String)>
where
    T: Send + 'static,
    F: FnOnce(&mut LogDatabase) -> Result<T, rusqlite::Error> + Send + 'static,
{
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        let mut guard = db.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        f(&mut guard)
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Błąd wykonania zapytania do bazy: {e}"),
        )
    })?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn get_status(
    State(state): State<ApiState>,
) -> Result<Json<StatusResponse>, (StatusCode, String)> {
    let total_qsos = with_db(&state, move |db| db.count_all()).await?;
    let (rig_connected, frequency_hz, mode) = {
        let rig = state
            .rig_state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        (rig.connected, rig.frequency_hz, rig.mode.clone())
    };

    Ok(Json(StatusResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        adif_version: ADIF_VERSION.to_string(),
        callsign: state.callsign,
        total_qsos,
        uptime_secs: state.start_time.elapsed().as_secs(),
        rig_connected,
        frequency_hz,
        mode,
    }))
}

async fn get_endpoints() -> Json<Vec<EndpointDoc>> {
    Json(vec![
        EndpointDoc {
            method: "GET",
            path: "/api/v1/status",
            auth_required: false,
            description: "Status serwera, wersja programu i ADIF, liczba QSO, stan CAT.",
        },
        EndpointDoc {
            method: "GET",
            path: "/api/v1/endpoints",
            auth_required: false,
            description: "Lista wszystkich dostępnych endpointów REST i WebSocket API v1.",
        },
        EndpointDoc {
            method: "GET",
            path: "/api/v1/qsos",
            auth_required: true,
            description: "Paginowana i filtrowana lista łączności (limit, offset, callsign, band, mode, date_from, date_to, journal_id, lotw_confirmed, eqsl_confirmed, qsl_rcvd).",
        },
        EndpointDoc {
            method: "POST",
            path: "/api/v1/qsos",
            auth_required: true,
            description: "Dodaje nowe QSO z automatycznym rozpoznaniem DXCC, aktualizacją dyplomów i powiadomieniem WebSocket.",
        },
        EndpointDoc {
            method: "GET",
            path: "/api/v1/qsos/{id}",
            auth_required: true,
            description: "Pobiera pojedynczy rekord QSO po identyfikatorze ID.",
        },
        EndpointDoc {
            method: "PUT",
            path: "/api/v1/qsos/{id}",
            auth_required: true,
            description: "Aktualizuje istniejący rekord QSO po identyfikatorze ID.",
        },
        EndpointDoc {
            method: "DELETE",
            path: "/api/v1/qsos/{id}",
            auth_required: true,
            description: "Usuwa rekord QSO z bazy danych po identyfikatorze ID.",
        },
        EndpointDoc {
            method: "GET",
            path: "/api/v1/qsos/callsign/{call}",
            auth_required: true,
            description: "Historia łączności z danym znakiem wywoławczym oraz sprawdzanie duplikatów (?band=20m&mode=CW&same_day=true).",
        },
        EndpointDoc {
            method: "GET",
            path: "/api/v1/lookup/{call}",
            auth_required: true,
            description: "Rozpoznanie DXCC, stref CQ/ITU, okręgu SP, klubów oraz statusu dyplomowego ATNO dla znaku (?band=20m&mode=CW).",
        },
        EndpointDoc {
            method: "GET",
            path: "/api/v1/adif/export",
            auth_required: true,
            description: "Eksport dziennika (lub wyfiltrowanego zakresu) w standardzie ADIF 3.1.7.",
        },
        EndpointDoc {
            method: "POST",
            path: "/api/v1/adif/import",
            auth_required: true,
            description: "Masowy import łączności z przesłanego tekstu ADIF wraz z raportem błędów.",
        },
        EndpointDoc {
            method: "GET",
            path: "/api/v1/rig",
            auth_required: true,
            description: "Aktualny stan transceivera CAT / VFO (częstotliwość, pasmo, emisja, S-Meter, Split, PTT).",
        },
        EndpointDoc {
            method: "POST",
            path: "/api/v1/rig",
            auth_required: true,
            description: "Zdalne sterowanie częstotliwością, emisją, VFO, Split i PTT transceivera.",
        },
        EndpointDoc {
            method: "GET",
            path: "/api/v1/awards",
            auth_required: true,
            description: "Podsumowanie postępów dyplomowych (DXCC, WAZ, WAC, WAS, WPX, VUCC, IOTA, SOTA, POTA, PGA).",
        },
        EndpointDoc {
            method: "GET",
            path: "/api/v1/journals",
            auth_required: true,
            description: "Lista profili dzienników (np. DEFAULT, CONTEST, PORTABLE) oraz aktywny dziennik.",
        },
        EndpointDoc {
            method: "GET",
            path: "/api/v1/stats",
            auth_required: true,
            description: "Statystyki logu (liczba QSO, unikalne znaki i podmioty DXCC, podział na pasma, emisje, kontynenty, QSL).",
        },
        EndpointDoc {
            method: "GET",
            path: "/api/v1/cluster/spots",
            auth_required: true,
            description: "Bieżące spoty DX Cluster z filtrowaniem (?limit=50&band=20m&ft8=false&call=SP).",
        },
        EndpointDoc {
            method: "GET",
            path: "/api/v1/ws",
            auth_required: true,
            description: "Strumień WebSocket na żywo (zdarzenia QsoLogged, DxSpot, RigState, ClusterStatus, CloudSync, Toast).",
        },
    ])
}

fn has_advanced_filter(query: &QsoQuery) -> bool {
    query
        .callsign
        .as_ref()
        .is_some_and(|s| !s.trim().is_empty())
        || query.band.as_ref().is_some_and(|s| !s.trim().is_empty())
        || query.mode.as_ref().is_some_and(|s| !s.trim().is_empty())
        || query
            .date_from
            .as_ref()
            .is_some_and(|s| !s.trim().is_empty())
        || query.date_to.as_ref().is_some_and(|s| !s.trim().is_empty())
        || query
            .journal_id
            .as_ref()
            .is_some_and(|s| !s.trim().is_empty())
        || query.lotw_confirmed.is_some()
        || query.eqsl_confirmed.is_some()
        || query.qsl_rcvd.is_some()
}

fn build_advanced_filter(query: &QsoQuery) -> AdvancedQsoFilter {
    let mut filter = AdvancedQsoFilter::default();
    if let Some(ref c) = query.callsign {
        let trimmed = c.trim();
        if !trimmed.is_empty() {
            filter.callsign_query = Some(trimmed.to_string());
        }
    }
    if let Some(ref b) = query.band {
        for part in b.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            filter.bands.push(part.to_string());
        }
    }
    if let Some(ref m) = query.mode {
        for part in m
            .split(',')
            .map(|s| s.trim().to_uppercase())
            .filter(|s| !s.is_empty())
        {
            filter.modes.push(part);
        }
    }
    filter.date_from.clone_from(&query.date_from);
    filter.date_to.clone_from(&query.date_to);
    filter.journal_id.clone_from(&query.journal_id);
    filter.lotw_confirmed = query.lotw_confirmed;
    filter.eqsl_confirmed = query.eqsl_confirmed;
    filter.qsl_rcvd = query.qsl_rcvd;
    filter
}

async fn get_qsos(
    State(state): State<ApiState>,
    Query(query): Query<QsoQuery>,
) -> Result<Json<Vec<QsoRecord>>, (StatusCode, String)> {
    let limit = clamp_usize(query.limit, 50, MAX_QSO_LIMIT);
    let offset = clamp_usize(query.offset, 0, MAX_QSO_OFFSET);
    let advanced = has_advanced_filter(&query);

    let qsos = with_db(&state, move |db| {
        if advanced {
            let filter = build_advanced_filter(&query);
            let mut qsos = db.search_qsos_advanced(&filter)?;
            if offset < qsos.len() {
                qsos.drain(0..offset);
            } else {
                qsos.clear();
            }
            qsos.truncate(limit);
            Ok(qsos)
        } else {
            db.get_qsos_paginated(limit, offset)
        }
    })
    .await?;

    Ok(Json(qsos))
}

async fn get_qso_by_id(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
) -> Result<Json<QsoRecord>, (StatusCode, String)> {
    let qso = with_db(&state, move |db| db.get_qso_by_id(id)).await?;

    match qso {
        Some(q) => Ok(Json(q)),
        None => Err((
            StatusCode::NOT_FOUND,
            format!("Nie znaleziono QSO o ID {id}"),
        )),
    }
}

/// Normalizuje i wzbogaca rekord QSO przed zapisem/aktualizacją przez REST API.
fn normalize_and_enrich_qso(
    qso: &mut QsoRecord,
    prefix_matcher: &PrefixMatcher,
) -> Result<(), (StatusCode, String)> {
    qso.callsign = qso.callsign.trim().to_uppercase();
    if qso.callsign.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Pole 'callsign' nie może być puste.".to_string(),
        ));
    }
    qso.band = qso.band.trim().to_string();
    if qso.band.is_empty() {
        if let Some(f_mhz) = qso.freq {
            qso.band = crate::core::bandplan::freq_khz_to_band(f_mhz * 1000.0)
                .unwrap_or("20m")
                .to_string();
        } else {
            qso.band = "20m".to_string();
        }
    }
    qso.mode = qso.mode.trim().to_uppercase();
    if qso.mode.is_empty() {
        qso.mode = "CW".to_string();
    }
    let now = chrono::Utc::now();
    if qso.qso_date.trim().is_empty() {
        qso.qso_date = now.format("%Y%m%d").to_string();
    } else {
        qso.qso_date = qso.adif_date();
    }
    if qso.time_on.trim().is_empty() {
        qso.time_on = now.format("%H%M%S").to_string();
    } else {
        qso.time_on = qso.adif_time();
    }

    // Wzbogać brakujące pola DXCC z bazy prefiksów
    if (qso.dxcc.is_none() || qso.country.is_none()) && !qso.callsign.is_empty() {
        if let Some(info) = prefix_matcher.lookup(&qso.callsign) {
            if qso.dxcc.is_none() {
                qso.dxcc = Some(info.dxcc);
            }
            if qso.country.is_none() {
                qso.country = Some(info.country);
            }
            if qso.continent.is_none() {
                qso.continent = Some(info.continent);
            }
            if qso.cqz.is_none() {
                qso.cqz = Some(info.cqz);
            }
            if qso.ituz.is_none() {
                qso.ituz = Some(info.ituz);
            }
        }
    }

    Ok(())
}

async fn post_qso(
    State(state): State<ApiState>,
    Json(qso): Json<QsoRecord>,
) -> Result<(StatusCode, Json<QsoRecord>), (StatusCode, String)> {
    let ingestor = state.ingestor.clone();
    let outcome = tokio::task::spawn_blocking(move || ingestor.ingest(qso, QsoOrigin::RestApi))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Błąd zadania asynchronicznego: {e}"),
            )
        })?
        .map_err(|err| match err {
            IngestError::EmptyCallsign => (StatusCode::BAD_REQUEST, err.msg().text(Language::Pl)),
            IngestError::Invalid(issue) => {
                (StatusCode::BAD_REQUEST, issue.msg().text(Language::Pl))
            }
            IngestError::Duplicate => (StatusCode::CONFLICT, err.msg().text(Language::Pl)),
            IngestError::Storage(error) => {
                eprintln!("[REST API] Błąd zapisu QSO do bazy danych: {error}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Błąd zapisu QSO do bazy danych.".to_string(),
                )
            }
        })?;

    state.reload_flag.store(true, Ordering::Release);

    Ok((StatusCode::CREATED, Json(outcome.qso)))
}

async fn put_qso_by_id(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
    Json(mut qso): Json<QsoRecord>,
) -> Result<Json<QsoRecord>, (StatusCode, String)> {
    normalize_and_enrich_qso(&mut qso, &state.prefix_matcher)?;
    qso.validate()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.msg().text(Language::Pl)))?;

    let qso_for_db = qso.clone();
    let found = with_db(&state, move |db| {
        let existing = db.get_qso_by_id(id)?;
        if existing.is_none() {
            return Ok(false);
        }
        db.update_qso(id, &qso_for_db)?;
        Ok(true)
    })
    .await?;
    if !found {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Nie znaleziono QSO o ID {id}"),
        ));
    }

    qso.id = Some(id);
    state.reload_flag.store(true, Ordering::Release);
    Ok(Json(qso))
}

async fn delete_qso_by_id(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let found = with_db(&state, move |db| {
        let existing = db.get_qso_by_id(id)?;
        if existing.is_none() {
            return Ok(false);
        }
        db.delete_qso(id)?;
        Ok(true)
    })
    .await?;
    if !found {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Nie znaleziono QSO o ID {id}"),
        ));
    }

    state.reload_flag.store(true, Ordering::Release);
    Ok(Json(serde_json::json!({
        "deleted": true,
        "id": id,
    })))
}

async fn get_qsos_by_callsign(
    State(state): State<ApiState>,
    Path(call): Path<String>,
    Query(query): Query<DupeCheckQuery>,
) -> Result<Json<CallsignHistoryResponse>, (StatusCode, String)> {
    let clean_call = call.trim().to_uppercase();
    let clean_call_for_db = clean_call.clone();
    let previous_qsos =
        with_db(&state, move |db| db.find_previous_qsos(&clean_call_for_db)).await?;

    let today = chrono::Utc::now().format("%Y%m%d").to_string();
    let is_dupe = previous_qsos.iter().any(|q| {
        let band_match = query
            .band
            .as_ref()
            .is_none_or(|b| q.band.eq_ignore_ascii_case(b.trim()));
        let mode_match = query
            .mode
            .as_ref()
            .is_none_or(|m| q.mode.eq_ignore_ascii_case(m.trim()));
        let day_match = if query.same_day.unwrap_or(false) {
            q.adif_date() == today
        } else {
            true
        };
        band_match && mode_match && day_match
    });

    Ok(Json(CallsignHistoryResponse {
        callsign: clean_call,
        worked_before: !previous_qsos.is_empty(),
        total_qsos: previous_qsos.len(),
        is_dupe,
        previous_qsos,
    }))
}

async fn lookup_callsign(
    State(state): State<ApiState>,
    Path(call): Path<String>,
    Query(query): Query<LookupQuery>,
) -> Result<Json<LookupResponse>, (StatusCode, String)> {
    let clean = call.trim().to_uppercase();
    if clean.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Pusty znak wywoławczy".to_string()));
    }

    let band = query.band.as_deref().unwrap_or("20m");
    let mode = query.mode.as_deref().unwrap_or("CW").to_uppercase();

    let prefix_info = state.prefix_matcher.lookup(&clean);
    let wpx_prefix = crate::core::prefix::extract_wpx_prefix(&clean);
    let polish_district = crate::core::awards::extract_sp_district(&clean);
    let clubs = ClubRegistry::check(&clean);

    let award_status = prefix_info.as_ref().map(|info| {
        let awards = state
            .awards_engine
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        awards.check_status_full(
            &clean,
            band,
            &mode,
            Some(info.dxcc),
            None,
            Some(info.cqz),
            None,
            Some(&info.continent),
            None,
        )
    });

    let clean_for_db = clean.clone();
    let previous_qso_count = with_db(&state, move |db| {
        Ok(db.find_previous_qsos(&clean_for_db).map_or(0, |v| v.len()))
    })
    .await?;

    Ok(Json(LookupResponse {
        callsign: clean,
        wpx_prefix,
        dxcc: prefix_info.as_ref().map(|i| i.dxcc),
        country: prefix_info.as_ref().map(|i| i.country.clone()),
        continent: prefix_info.as_ref().map(|i| i.continent.clone()),
        cqz: prefix_info.as_ref().map(|i| i.cqz),
        ituz: prefix_info.as_ref().map(|i| i.ituz),
        latitude: prefix_info.as_ref().map(|i| i.latitude),
        longitude: prefix_info.as_ref().map(|i| i.longitude),
        polish_district,
        clubs,
        award_status,
        previous_qso_count,
    }))
}

async fn export_adif_handler(
    State(state): State<ApiState>,
    Query(query): Query<QsoQuery>,
) -> Result<Response, (StatusCode, String)> {
    let qsos = with_db(&state, move |db| {
        if has_advanced_filter(&query) {
            let filter = build_advanced_filter(&query);
            let mut list = db.search_qsos_advanced(&filter)?;
            if let Some(lim) = query.limit {
                list.truncate(lim);
            }
            Ok(list)
        } else if let Some(lim) = query.limit {
            db.get_qsos_paginated(lim, query.offset.unwrap_or(0))
        } else {
            db.get_all_qsos()
        }
    })
    .await?;

    let adif_text = adif::export_adif(&qsos, "SPLogbook", &state.callsign);
    let mut resp = adif_text.into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    resp.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"splogbook_export.adi\""),
    );
    Ok(resp)
}

async fn import_adif_handler(
    State(state): State<ApiState>,
    body: String,
) -> Result<Json<AdifImportApiResponse>, (StatusCode, String)> {
    if body.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Treść żądania ADIF jest pusta.".to_string(),
        ));
    }

    let ingestor = state.ingestor.clone();
    let db = Arc::clone(&state.db);
    let task_result = tokio::task::spawn_blocking(move || {
        let parse_report = adif::parse_adif_with_report(&body);
        let qsos = parse_report.qsos;
        let rejected = parse_report.rejected;
        let errors = parse_report.errors;
        let options = ImportOptions {
            journal_id: "DEFAULT".to_string(),
            enrich: true,
        };
        let import_report = ingestor.import_batch(qsos, &options)?;
        let total_qsos = {
            let db_guard = db.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            db_guard.count_all().unwrap_or(import_report.inserted)
        };
        Ok::<_, IngestError>((rejected, errors, import_report, total_qsos))
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Błąd zadania asynchronicznego: {e}"),
        )
    })?;

    match task_result {
        Ok((adif_rejected, mut errors, import_report, total_qsos)) => {
            state.reload_flag.store(true, Ordering::Release);
            if let Some(issue) = import_report.first_issue {
                errors.push(issue.msg().text(Language::Pl));
            }
            if import_report.skipped_duplicates > 0 {
                errors.push("duplikat (rekord już istnieje w dzienniku)".to_string());
            }
            let rejected =
                adif_rejected + import_report.rejected_invalid + import_report.skipped_duplicates;
            Ok(Json(AdifImportApiResponse {
                imported: import_report.inserted,
                rejected,
                errors,
                total_qsos,
            }))
        }
        Err(IngestError::Storage(e)) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Błąd bazy danych podczas importu: {e}"),
        )),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            format!("Błąd walidacji importu: {e}"),
        )),
    }
}

async fn get_rig_state(State(state): State<ApiState>) -> Json<RigStateResponse> {
    let rig = state
        .rig_state
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    let band = crate::core::bandplan::get_band_by_freq(rig.frequency_hz)
        .map_or_else(|| "OTHER".to_string(), |b| b.name.to_string());
    Json(RigStateResponse {
        connected: rig.connected,
        frequency_hz: rig.frequency_hz,
        frequency_mhz: (rig.frequency_hz as f64) / 1_000_000.0,
        band,
        mode: rig.mode,
        passband_hz: rig.passband_hz,
        vfo: rig.vfo,
        split_enabled: rig.split_enabled,
        s_meter_dbm: rig.s_meter_dbm,
        s_meter_unit: rig.s_meter_unit,
        rf_power_watts: rig.rf_power_watts,
        ptt: rig.ptt,
    })
}

async fn post_rig_control(
    State(state): State<ApiState>,
    Json(req): Json<RigControlRequest>,
) -> Json<RigStateResponse> {
    let updated = {
        let mut rig = state
            .rig_state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(freq) = req.frequency_hz {
            rig.frequency_hz = freq;
        }
        if let Some(mode) = req.mode {
            rig.mode = mode.trim().to_uppercase();
        }
        if let Some(pb) = req.passband_hz {
            rig.passband_hz = pb;
        }
        if let Some(vfo) = req.vfo {
            rig.vfo = vfo.trim().to_uppercase();
        }
        if let Some(split) = req.split_enabled {
            rig.split_enabled = split;
        }
        if let Some(ptt) = req.ptt {
            rig.ptt = ptt;
        }
        rig.clone()
    };

    state.events.publish(AppEvent::RigState {
        frequency_hz: updated.frequency_hz,
        mode: updated.mode.clone(),
        connected: updated.connected,
    });

    let band = crate::core::bandplan::get_band_by_freq(updated.frequency_hz)
        .map_or_else(|| "OTHER".to_string(), |b| b.name.to_string());

    Json(RigStateResponse {
        connected: updated.connected,
        frequency_hz: updated.frequency_hz,
        frequency_mhz: (updated.frequency_hz as f64) / 1_000_000.0,
        band,
        mode: updated.mode,
        passband_hz: updated.passband_hz,
        vfo: updated.vfo,
        split_enabled: updated.split_enabled,
        s_meter_dbm: updated.s_meter_dbm,
        s_meter_unit: updated.s_meter_unit,
        rf_power_watts: updated.rf_power_watts,
        ptt: updated.ptt,
    })
}

async fn get_awards_summary(State(state): State<ApiState>) -> Json<AwardsSummaryResponse> {
    let awards = state
        .awards_engine
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Json(AwardsSummaryResponse {
        dxcc_worked: awards.worked_dxcc_all.len(),
        dxcc_confirmed: awards.confirmed_dxcc.len(),
        waz_worked: awards.worked_waz.len(),
        waz_confirmed: awards.confirmed_waz.len(),
        wac_worked: awards.worked_wac.len(),
        wac_confirmed: awards.confirmed_wac.len(),
        was_worked: awards.worked_was.len(),
        was_confirmed: awards.confirmed_was.len(),
        wpx_worked: awards.worked_wpx.len(),
        vucc_grids_worked: awards.worked_vucc.len(),
        iota_worked: awards.worked_iota.len(),
        sota_worked: awards.worked_sota.len(),
        pota_worked: awards.worked_pota.len(),
        pga_worked: awards.worked_pga.len(),
        sp_districts_worked: awards.worked_sp_districts.len(),
        sp_districts_confirmed: awards.confirmed_sp_districts.len(),
    })
}

async fn get_journals(
    State(state): State<ApiState>,
) -> Result<Json<JournalsResponse>, (StatusCode, String)> {
    let (journals, active_journal_id) = with_db(&state, move |db| {
        let journals = db.get_all_journals()?;
        let active_journal_id = db
            .get_active_journal()
            .map_or_else(|_| "DEFAULT".to_string(), |j| j.id);
        Ok((journals, active_journal_id))
    })
    .await?;
    Ok(Json(JournalsResponse {
        active_journal_id,
        journals,
    }))
}

async fn get_stats(
    State(state): State<ApiState>,
) -> Result<Json<StatsResponse>, (StatusCode, String)> {
    let stats = with_db(&state, |db| {
        let total = db.count_all()?;
        let (unique_calls, unique_dxcc) = db.stats_unique_counts()?;

        let band_stats = db.stats_qso_per_band()?;
        let mut by_band = HashMap::new();
        for (band, count) in band_stats {
            by_band.insert(band, count);
        }

        let mode_stats = db.stats_qso_per_mode()?;
        let mut by_mode = HashMap::new();
        for (mode, count) in mode_stats {
            by_mode.insert(mode, count);
        }

        let cont_stats = db.stats_qso_per_continent()?;
        let mut by_continent = HashMap::new();
        for (cont, count) in cont_stats {
            by_continent.insert(cont, count);
        }

        let (_tot, qsl_lotw, qsl_eqsl, qsl_paper) = db.stats_qsl_summary()?;

        Ok(StatsResponse {
            total,
            unique_calls,
            unique_dxcc,
            by_band,
            by_mode,
            by_continent,
            qsl_lotw,
            qsl_eqsl,
            qsl_paper,
        })
    })
    .await?;

    Ok(Json(stats))
}

/// GET /api/v1/cluster/spots?limit=N&band=20m&ft8=false&call=SP
/// Zwraca bieżące spoty DX Cluster z bufora aplikacji.
async fn get_cluster_spots(
    State(state): State<ApiState>,
    Query(query): Query<ClusterQuery>,
) -> Json<Vec<serde_json::Value>> {
    let limit = clamp_usize(query.limit, 50, MAX_SPOT_LIMIT);
    let band_filter = query.band.as_deref().map(|b| b.trim().to_lowercase());
    let call_filter = query.call.as_deref().map(|c| c.trim().to_uppercase());

    let spots = state
        .cluster_spots
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let result: Vec<serde_json::Value> = spots
        .iter()
        .filter(|s| {
            if let Some(ref b) = band_filter {
                if !b.is_empty() && s.band.to_lowercase() != *b {
                    return false;
                }
            }
            if let Some(ft8) = query.ft8 {
                if s.is_ft8 != ft8 {
                    return false;
                }
            }
            if let Some(ref c) = call_filter {
                if !c.is_empty() && !s.dx_call.to_uppercase().contains(c) {
                    return false;
                }
            }
            true
        })
        .take(limit)
        .map(|s| {
            serde_json::json!({
                "spotter": s.spotter,
                "dx_call": s.dx_call,
                "frequency_khz": s.frequency_khz,
                "band": s.band,
                "comment": s.comment,
                "time_utc": s.time_utc,
                "is_ft8": s.is_ft8,
                "is_skimmer": s.is_skimmer,
            })
        })
        .collect();
    Json(result)
}

/// GET /api/v1/ws
/// Uaktualnienie WebSocket strumieniujące zdarzenia aplikacji na żywo
/// (nowe QSO, spoty DX, zmiany stanu radia, status klastra/chmury).
/// Wymaga nagłówka `X-Api-Key` (jak pozostałe chronione endpointy).
async fn ws_handler(ws: WebSocketUpgrade, State(state): State<ApiState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: ApiState) {
    use futures_util::{SinkExt, StreamExt};

    let (mut sender, mut receiver) = socket.split();

    // Powitanie z wersją aplikacji — pozwala klientowi zweryfikować kompatybilność.
    let hello = serde_json::json!({
        "type": "hello",
        "version": env!("CARGO_PKG_VERSION"),
        "adif_version": ADIF_VERSION,
    })
    .to_string();
    if sender.send(Message::Text(hello.into())).await.is_err() {
        return;
    }

    let mut rx = state.events.subscribe();
    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Ok(ev) => {
                        let Ok(text) = serde_json::to_string(&ev) else {
                            continue;
                        };
                        if sender.send(Message::Text(text.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        let skip = serde_json::json!({ "type": "lagged", "skipped": skipped }).to_string();
                        if sender.send(Message::Text(skip.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Close(_)) | Err(_)) | None => break,
                    Some(Ok(Message::Ping(payload))) => {
                        if sender.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_) | Message::Text(_) | Message::Binary(_))) => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::Service;

    fn test_api_state() -> ApiState {
        let db = LogDatabase::open_in_memory().expect("in-memory db");
        ApiState::new(
            Arc::new(Mutex::new(db)),
            "SP6INA".to_string(),
            Arc::new(Mutex::new(Vec::new())),
            "secret-key-123".to_string(),
            EventBus::default(),
        )
    }

    async fn call_router(app: &Router, req: Request<Body>) -> Response {
        let mut svc = app.clone();
        Service::call(&mut svc, req).await.unwrap()
    }

    #[test]
    fn clamp_usize_applies_default_and_max_cap() {
        assert_eq!(clamp_usize(None, 50, MAX_QSO_LIMIT), 50);
        assert_eq!(clamp_usize(Some(10), 50, MAX_QSO_LIMIT), 10);
        assert_eq!(
            clamp_usize(Some(1_000_000), 50, MAX_QSO_LIMIT),
            MAX_QSO_LIMIT
        );
        assert_eq!(
            clamp_usize(Some(999_999), 50, MAX_SPOT_LIMIT),
            MAX_SPOT_LIMIT
        );
        assert_eq!(clamp_usize(Some(0), 50, MAX_QSO_LIMIT), 0);
    }

    #[test]
    fn app_event_serializes_for_websocket() {
        let ev = crate::core::events::AppEvent::DxSpot {
            spotter: "SP6INA".into(),
            dx_call: "DL1ABC".into(),
            frequency_khz: 14_250.0,
            band: "20m".into(),
            comment: "TNX".into(),
            time_utc: "1200".into(),
            is_ft8: false,
            is_skimmer: false,
        };
        let json = serde_json::to_string(&ev).expect("serializacja");
        assert!(json.contains("\"type\":\"dx_spot\""));
        assert!(json.contains("\"dx_call\":\"DL1ABC\""));
    }

    #[test]
    fn generate_api_key_and_constant_time_eq_work() {
        let k1 = generate_api_key();
        let k2 = generate_api_key();
        assert_eq!(k1.len(), 64);
        assert_ne!(k1, k2);
        assert!(k1.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(constant_time_eq(k1.as_bytes(), k1.as_bytes()));
        assert!(!constant_time_eq(k1.as_bytes(), k2.as_bytes()));
        assert!(!constant_time_eq(b"abc", b"abcd"));
    }

    #[tokio::test]
    async fn rest_api_crud_lookup_and_adif_roundtrip() {
        let state = test_api_state();
        let mut event_rx = state.events.subscribe();
        let app = build_api_router(state.clone());

        // 1. Publiczny /api/v1/status bez klucza
        let resp = call_router(
            &app,
            Request::builder()
                .uri("/api/v1/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);

        // 2. Chroniony endpoint bez klucza -> 401 UNAUTHORIZED
        let resp = call_router(
            &app,
            Request::builder()
                .uri("/api/v1/qsos")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        // 3. POST /api/v1/qsos — dodanie nowego QSO (z automatycznym rozpoznaniem DXCC i emisją zdarzenia)
        let new_qso_json = serde_json::json!({
            "callsign": "dl1abc",
            "band": "20m",
            "mode": "cw",
            "qso_date": "20260928",
            "time_on": "101500",
            "rst_sent": "599",
            "rst_rcvd": "599"
        })
        .to_string();

        let resp = call_router(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/qsos")
                .header("x-api-key", "secret-key-123")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(new_qso_json))
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::CREATED);

        // Zdarzenie powinno zostać opublikowane na EventBus, a flaga odświeżenia GUI ustawiona
        assert!(state.reload_flag.load(Ordering::Acquire));
        let ev = event_rx
            .try_recv()
            .expect("Powinno zostać wyemitowane zdarzenie QsoLogged");
        match ev {
            AppEvent::QsoLogged {
                callsign,
                band,
                mode,
                ..
            } => {
                assert_eq!(callsign, "DL1ABC");
                assert_eq!(band, "20m");
                assert_eq!(mode, "CW");
            }
            other => panic!("Nieoczekiwane zdarzenie: {other:?}"),
        }

        // 4. GET /api/v1/qsos/callsign/DL1ABC?band=20m&mode=CW -> dupe check
        let resp = call_router(
            &app,
            Request::builder()
                .uri("/api/v1/qsos/callsign/DL1ABC?band=20m&mode=CW")
                .header("x-api-key", "secret-key-123")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);

        // 5. GET /api/v1/lookup/SP6INA?band=40m&mode=SSB
        let resp = call_router(
            &app,
            Request::builder()
                .uri("/api/v1/lookup/SP6INA?band=40m&mode=SSB")
                .header("x-api-key", "secret-key-123")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);

        // 6. POST /api/v1/rig & GET /api/v1/rig
        let rig_req = serde_json::json!({
            "frequency_hz": 7_074_000,
            "mode": "FT8",
            "split_enabled": true
        })
        .to_string();
        let resp = call_router(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/rig")
                .header("x-api-key", "secret-key-123")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(rig_req))
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(state.rig_state.read().unwrap().frequency_hz, 7_074_000);

        // 7. DELETE /api/v1/qsos/1
        let resp = call_router(
            &app,
            Request::builder()
                .method(Method::DELETE)
                .uri("/api/v1/qsos/1")
                .header("x-api-key", "secret-key-123")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn rest_api_import_adif_empty_body_rejected() {
        let state = test_api_state();
        let app = build_api_router(state);

        let resp = call_router(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/adif/import")
                .header("x-api-key", "secret-key-123")
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from("   \n\t"))
                .unwrap(),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let msg = String::from_utf8_lossy(&bytes);
        assert!(msg.contains("Treść żądania ADIF jest pusta."));
    }

    #[tokio::test]
    async fn rest_api_import_adif_success_duplicates_and_scp_update() {
        let state = test_api_state();
        let app = build_api_router(state.clone());

        let adif = "<CALL:6>SP1ABC <BAND:3>20m <MODE:2>CW <QSO_DATE:8>20261012 <TIME_ON:6>120000 <RST_SENT:3>599 <RST_RCVD:3>599 <EOR>\n\
                    <CALL:6>DL2DEF <BAND:3>40m <MODE:3>SSB <QSO_DATE:8>20261012 <TIME_ON:6>121500 <RST_SENT:2>59 <RST_RCVD:2>59 <EOR>";

        // Pierwszy import - 2 rekordy
        let resp = call_router(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/adif/import")
                .header("x-api-key", "secret-key-123")
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from(adif))
                .unwrap(),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["imported"], 2);
        assert_eq!(json["rejected"], 0);
        assert_eq!(json["total_qsos"], 2);

        // Flaga odświeżenia GUI
        assert!(state.reload_flag.load(Ordering::Acquire));

        // SCP zaktualizowany
        {
            let scp = state.scp_engine.lock().unwrap();
            let matches = scp.search("SP1ABC", 5);
            assert!(matches.contains(&"SP1ABC".to_string()));
            let matches_dl = scp.search("DL2DEF", 5);
            assert!(matches_dl.contains(&"DL2DEF".to_string()));
        }

        // Awards zaktualizowane
        {
            let awards = state.awards_engine.lock().unwrap();
            assert!(
                awards
                    .worked_calls
                    .iter()
                    .any(|(call, _, _)| call == "SP1ABC")
            );
            assert!(
                awards
                    .worked_calls
                    .iter()
                    .any(|(call, _, _)| call == "DL2DEF")
            );
        }

        // Drugi import - duplikaty
        let resp_dup = call_router(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/adif/import")
                .header("x-api-key", "secret-key-123")
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from(adif))
                .unwrap(),
        )
        .await;

        assert_eq!(resp_dup.status(), StatusCode::OK);
        let bytes_dup = axum::body::to_bytes(resp_dup.into_body(), usize::MAX)
            .await
            .unwrap();
        let json_dup: serde_json::Value = serde_json::from_slice(&bytes_dup).unwrap();
        assert_eq!(json_dup["imported"], 0);
        assert_eq!(json_dup["rejected"], 2);
        assert_eq!(json_dup["total_qsos"], 2);
        let errors = json_dup["errors"].as_array().unwrap();
        assert!(!errors.is_empty());
        assert!(
            errors
                .iter()
                .any(|e| e.as_str().unwrap().contains("duplikat"))
        );
    }

    #[tokio::test]
    async fn rest_api_import_adif_malformed_records() {
        let state = test_api_state();
        let app = build_api_router(state);

        // Rekord ze składnią uszkodzoną (brak zamknięcia tagu) oraz rekord bez CALL
        let malformed_adif = "<CALL:6>SP1ABC <BAND:3>20m <MODE:2>CW <QSO_DATE:8>20261012 <TIME_ON:6>120000 <EOR>\n<CALL:6>SP2XYZ <BAND";

        let resp = call_router(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/adif/import")
                .header("x-api-key", "secret-key-123")
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from(malformed_adif))
                .unwrap(),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        // Pierwszy poprawny rekord powinien wejść
        assert_eq!(json["imported"], 1);

        // Rekord bez znaku wywoławczego (odrzucony przez walidację)
        let invalid_record_adif =
            "<BAND:3>20m <MODE:2>CW <QSO_DATE:8>20261012 <TIME_ON:6>120000 <EOR>";
        let resp2 = call_router(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/adif/import")
                .header("x-api-key", "secret-key-123")
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from(invalid_record_adif))
                .unwrap(),
        )
        .await;

        assert_eq!(resp2.status(), StatusCode::OK);
        let bytes2 = axum::body::to_bytes(resp2.into_body(), usize::MAX)
            .await
            .unwrap();
        let json2: serde_json::Value = serde_json::from_slice(&bytes2).unwrap();
        assert_eq!(json2["imported"], 0);
        assert_eq!(json2["rejected"], 1);
        let errors = json2["errors"].as_array().unwrap();
        assert!(!errors.is_empty());
    }

    #[tokio::test]
    async fn rest_api_post_qso_success_persists_and_registers_side_effects() {
        let state = test_api_state();
        let mut event_rx = state.events.subscribe();
        let app = build_api_router(state.clone());

        assert!(!state.reload_flag.load(Ordering::Acquire));

        let qso_payload = serde_json::json!({
            "callsign": "SP1ABC",
            "band": "20m",
            "mode": "CW",
            "qso_date": "20261012",
            "time_on": "120000",
            "rst_sent": "599",
            "rst_rcvd": "599",
            "freq": 14.025
        })
        .to_string();

        let resp = call_router(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/qsos")
                .header("x-api-key", "secret-key-123")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(qso_payload))
                .unwrap(),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::CREATED);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: QsoRecord = serde_json::from_slice(&bytes).unwrap();
        let qso_id = created.id.expect("ID should be assigned");
        assert!(qso_id > 0);
        assert_eq!(created.callsign, "SP1ABC");

        // 1. Rekord w bazie danych
        {
            let db = state.db.lock().unwrap();
            let stored = db
                .get_qso_by_id(qso_id)
                .expect("db query failed")
                .expect("qso must exist");
            assert_eq!(stored.callsign, "SP1ABC");
            assert_eq!(stored.band, "20m");
            assert_eq!(stored.mode, "CW");
        }

        // 2. Aktualizacja AwardsEngine
        {
            let awards = state.awards_engine.lock().unwrap();
            assert!(
                awards
                    .worked_calls
                    .iter()
                    .any(|(call, band, mode)| call == "SP1ABC" && band == "20m" && mode == "CW")
            );
        }

        // 3. Aktualizacja ScpEngine
        {
            let scp = state.scp_engine.lock().unwrap();
            let matches = scp.search("SP1ABC", 5);
            assert!(matches.contains(&"SP1ABC".to_string()));
        }

        // 4. Publikacja zdarzenia AppEvent::QsoLogged dokładnie raz
        let ev = event_rx
            .try_recv()
            .expect("QsoLogged event must be published");
        match ev {
            AppEvent::QsoLogged {
                callsign,
                band,
                mode,
                frequency_hz,
                time_utc,
            } => {
                assert_eq!(callsign, "SP1ABC");
                assert_eq!(band, "20m");
                assert_eq!(mode, "CW");
                assert_eq!(frequency_hz, 14_025_000);
                assert_eq!(time_utc, "120000");
            }
            other => panic!("Unexpected event: {other:?}"),
        }
        assert!(event_rx.try_recv().is_err(), "Expected exactly one event");

        // 5. Reload flag
        assert!(state.reload_flag.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn rest_api_post_qso_normalizes_callsign_and_mode() {
        let state = test_api_state();
        let app = build_api_router(state.clone());

        let qso_payload = serde_json::json!({
            "callsign": "  dl2def  ",
            "band": "40m",
            "mode": "  ssb  ",
            "qso_date": "20261012",
            "time_on": "130000",
            "rst_sent": "59",
            "rst_rcvd": "59"
        })
        .to_string();

        let resp = call_router(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/qsos")
                .header("x-api-key", "secret-key-123")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(qso_payload))
                .unwrap(),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::CREATED);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: QsoRecord = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(created.callsign, "DL2DEF");
        assert_eq!(created.mode, "SSB");

        let db = state.db.lock().unwrap();
        let stored = db.get_qso_by_id(created.id.unwrap()).unwrap().unwrap();
        assert_eq!(stored.callsign, "DL2DEF");
        assert_eq!(stored.mode, "SSB");
    }

    #[tokio::test]
    async fn rest_api_post_qso_derives_band_from_freq() {
        let state = test_api_state();
        let app = build_api_router(state.clone());

        let qso_payload = serde_json::json!({
            "callsign": "DL3GHI",
            "band": "",
            "freq": 14.025,
            "mode": "CW",
            "qso_date": "20261012",
            "time_on": "140000",
            "rst_sent": "599",
            "rst_rcvd": "599"
        })
        .to_string();

        let resp = call_router(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/qsos")
                .header("x-api-key", "secret-key-123")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(qso_payload))
                .unwrap(),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::CREATED);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: QsoRecord = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(created.band, "20m");

        let db = state.db.lock().unwrap();
        let stored = db.get_qso_by_id(created.id.unwrap()).unwrap().unwrap();
        assert_eq!(stored.band, "20m");
    }

    #[tokio::test]
    async fn rest_api_post_qso_empty_callsign_rejected_without_side_effects() {
        let state = test_api_state();
        let mut event_rx = state.events.subscribe();
        let app = build_api_router(state.clone());

        assert!(!state.reload_flag.load(Ordering::Acquire));

        let qso_payload = serde_json::json!({
            "callsign": "   ",
            "band": "20m",
            "mode": "CW",
            "qso_date": "20261012",
            "time_on": "120000"
        })
        .to_string();

        let resp = call_router(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/qsos")
                .header("x-api-key", "secret-key-123")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(qso_payload))
                .unwrap(),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        // Brak zapisu w bazie
        assert_eq!(state.db.lock().unwrap().count_all().unwrap(), 0);
        // Brak wpisu w Awards
        assert!(state.awards_engine.lock().unwrap().worked_calls.is_empty());
        // Brak wpisu w SCP
        assert!(state.scp_engine.lock().unwrap().search("", 5).is_empty());
        // Brak zdarzenia w EventBus
        assert!(event_rx.try_recv().is_err());
        // Reload flag pozostaje false
        assert!(!state.reload_flag.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn rest_api_post_qso_missing_date_rejected_without_fabrication() {
        let state = test_api_state();
        let mut event_rx = state.events.subscribe();
        let app = build_api_router(state.clone());

        assert!(!state.reload_flag.load(Ordering::Acquire));

        let qso_payload = serde_json::json!({
            "callsign": "SP5XYZ",
            "band": "20m",
            "mode": "CW",
            "qso_date": "",
            "time_on": "120000"
        })
        .to_string();

        let resp = call_router(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/qsos")
                .header("x-api-key", "secret-key-123")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(qso_payload))
                .unwrap(),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        // Brak autogeneracji Utc::now() i brak zapisu w bazie
        assert_eq!(state.db.lock().unwrap().count_all().unwrap(), 0);
        // Brak efektów ubocznych
        assert!(state.awards_engine.lock().unwrap().worked_calls.is_empty());
        assert!(
            state
                .scp_engine
                .lock()
                .unwrap()
                .search("SP5XYZ", 5)
                .is_empty()
        );
        assert!(event_rx.try_recv().is_err());
        assert!(!state.reload_flag.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn rest_api_post_qso_missing_time_rejected_without_fabrication() {
        let state = test_api_state();
        let mut event_rx = state.events.subscribe();
        let app = build_api_router(state.clone());

        assert!(!state.reload_flag.load(Ordering::Acquire));

        let qso_payload = serde_json::json!({
            "callsign": "SP5XYZ",
            "band": "20m",
            "mode": "CW",
            "qso_date": "20261012",
            "time_on": ""
        })
        .to_string();

        let resp = call_router(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/qsos")
                .header("x-api-key", "secret-key-123")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(qso_payload))
                .unwrap(),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        // Brak autogeneracji Utc::now() i brak zapisu w bazie
        assert_eq!(state.db.lock().unwrap().count_all().unwrap(), 0);
        // Brak efektów ubocznych
        assert!(state.awards_engine.lock().unwrap().worked_calls.is_empty());
        assert!(
            state
                .scp_engine
                .lock()
                .unwrap()
                .search("SP5XYZ", 5)
                .is_empty()
        );
        assert!(event_rx.try_recv().is_err());
        assert!(!state.reload_flag.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn rest_api_post_qso_dxcc_enrichment_from_prefix_matcher() {
        use std::sync::atomic::AtomicU64;
        static CNT: AtomicU64 = AtomicU64::new(1);
        let id = CNT.fetch_add(1, Ordering::Relaxed);
        let uri = format!("file:test_api_prefix_{id}?mode=memory&cache=shared");
        let conn = rusqlite::Connection::open_with_flags(
            &uri,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
                | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        )
        .unwrap();
        conn.execute_batch(
            "CREATE TABLE UniqueCalls (
                Callsign TEXT, Country TEXT, ARRLPrefix TEXT, DXCC TEXT,
                Continent TEXT, CQZone INTEGER, ITUZone INTEGER,
                Latitude TEXT, Longitude TEXT, Enable TEXT
            );
            CREATE TABLE CountryDataEx (
                Country TEXT, ARRLPrefix TEXT, DXCC INTEGER, Continent TEXT,
                CQZone INTEGER, ITUZone INTEGER, Latitude TEXT, Longitude TEXT,
                PrefixList TEXT, EndDate TEXT
            );
            CREATE TABLE Province (
                Country TEXT, ARRLPrefix TEXT, DXCC TEXT, Continent TEXT,
                CQZone INTEGER, ITUZone INTEGER, Latitude TEXT, Longitude TEXT,
                PrefixList TEXT, Comment TEXT, EndDate TEXT
            );
            INSERT INTO UniqueCalls VALUES
                ('SP6INA', 'Poland', 'SP', '269', 'EU', 15, 28, '51.1N', '17.0E', '1');",
        )
        .unwrap();
        let matcher = Arc::new(PrefixMatcher::load_from_db(&uri).unwrap());

        let base_state = test_api_state();
        let state = base_state.with_station_context(
            matcher,
            Arc::new(RwLock::new(RigState::default())),
            Arc::new(Mutex::new(AwardsEngine::new())),
            Arc::new(AtomicBool::new(false)),
        );
        let app = build_api_router(state.clone());

        let qso_payload = serde_json::json!({
            "callsign": "SP6INA",
            "band": "20m",
            "mode": "CW",
            "qso_date": "20261012",
            "time_on": "120000"
        })
        .to_string();

        let resp = call_router(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/qsos")
                .header("x-api-key", "secret-key-123")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(qso_payload))
                .unwrap(),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::CREATED);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: QsoRecord = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(created.dxcc, Some(269));
        assert_eq!(created.country, Some("Poland".to_string()));
        assert_eq!(created.continent, Some("EU".to_string()));
        assert_eq!(created.cqz, Some(15));
        assert_eq!(created.ituz, Some(28));
    }
}
