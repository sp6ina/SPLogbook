use axum::{
    extract::{Path, Query, State},
    http::{StatusCode, Method, header},
    response::Response,
    routing::get,
    Json, Router, middleware,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::collections::HashMap;

use crate::core::qso::QsoRecord;

use crate::cluster::telnet::DxSpot;

#[derive(Clone)]
pub struct ApiState {
    db: Arc<Mutex<crate::core::database::LogDatabase>>,
    callsign: String,
    start_time: Instant,
    pub cluster_spots: Arc<Mutex<Vec<DxSpot>>>,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub version: String,
    pub callsign: String,
    pub total_qsos: usize,
    pub uptime_secs: u64,
}

#[derive(Deserialize)]
pub struct QsoQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Serialize)]
pub struct StatsResponse {
    pub total: usize,
    pub by_band: HashMap<String, i64>,
    pub by_mode: HashMap<String, i64>,
    pub qsl_lotw: i64,
    pub qsl_eqsl: i64,
}

#[derive(Deserialize)]
pub struct ClusterQuery {
    pub limit: Option<usize>,
}

async fn cors_middleware(req: axum::extract::Request, next: axum::middleware::Next) -> Response {
    use axum::http::HeaderValue;
    const ORIGIN: HeaderValue  = HeaderValue::from_static("*");
    const METHODS: HeaderValue = HeaderValue::from_static("GET, POST, OPTIONS");
    const HEADERS: HeaderValue = HeaderValue::from_static("Content-Type, Authorization");

    if req.method() == Method::OPTIONS {
        let mut res = Response::new(axum::body::Body::empty());
        res.headers_mut().insert(header::ACCESS_CONTROL_ALLOW_ORIGIN,  ORIGIN);
        res.headers_mut().insert(header::ACCESS_CONTROL_ALLOW_METHODS, METHODS);
        res.headers_mut().insert(header::ACCESS_CONTROL_ALLOW_HEADERS, HEADERS);
        return res;
    }

    let mut res = next.run(req).await;
    res.headers_mut().insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
    res
}

pub async fn start_api_server(
    db: Arc<Mutex<crate::core::database::LogDatabase>>,
    callsign: String,
    port: u16,
    cluster_spots: Arc<Mutex<Vec<DxSpot>>>,
) {
    let state = ApiState {
        db,
        callsign,
        start_time: Instant::now(),
        cluster_spots,
    };

    let app = Router::new()
        .route("/api/v1/status", get(get_status))
        .route("/api/v1/qsos", get(get_qsos).post(post_qso))
        .route("/api/v1/qsos/:id", get(get_qso_by_id))
        .route("/api/v1/stats", get(get_stats))
        .route("/api/v1/cluster/spots", get(get_cluster_spots))
        .layer(middleware::from_fn(cors_middleware))
        .with_state(state);

    let bind_addr = format!("127.0.0.1:{}", port);
    let listener = match tokio::net::TcpListener::bind(&bind_addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[REST API] Nie mozna uruchomic serwera na {}: {}. \
                Zmien port w menu Narzedzia -> REST API lub zwolnij port.", bind_addr, e);
            return;
        }
    };
    eprintln!("[REST API] Serwer uruchomiony na http://{}", bind_addr);
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("[REST API] Blad serwera: {}", e);
    }
}

async fn get_status(State(state): State<ApiState>) -> Result<Json<StatusResponse>, (StatusCode, String)> {
    let total_qsos = {
        let db = state.db.lock().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        db.count_all().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    };

    Ok(Json(StatusResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        callsign: state.callsign,
        total_qsos,
        uptime_secs: state.start_time.elapsed().as_secs(),
    }))
}

async fn get_qsos(
    State(state): State<ApiState>,
    Query(query): Query<QsoQuery>,
) -> Result<Json<Vec<QsoRecord>>, (StatusCode, String)> {
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let db = state.db.lock().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    // We fetch all or limit, then apply offset. A real implementation might use SQL LIMIT/OFFSET.
    // For now, we'll fetch recent and slice, or just use get_all_qsos.
    let mut qsos = db.get_all_qsos().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    if offset < qsos.len() {
        qsos.drain(0..offset);
    } else {
        qsos.clear();
    }
    
    qsos.truncate(limit);
    
    Ok(Json(qsos))
}

async fn get_qso_by_id(
    State(state): State<ApiState>,
    Path(id): Path<i64>,
) -> Result<Json<QsoRecord>, (StatusCode, String)> {
    let db = state.db.lock().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let qso = db.get_qso_by_id(id).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    match qso {
        Some(q) => Ok(Json(q)),
        None => Err((StatusCode::NOT_FOUND, "QSO not found".to_string())),
    }
}

async fn post_qso(
    State(state): State<ApiState>,
    Json(mut qso): Json<QsoRecord>,
) -> Result<Json<QsoRecord>, (StatusCode, String)> {
    let db = state.db.lock().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let id = db.insert_qso(&qso).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    qso.id = Some(id);
    Ok(Json(qso))
}

async fn get_stats(State(state): State<ApiState>) -> Result<Json<StatsResponse>, (StatusCode, String)> {
    let db = state.db.lock().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    let total = db.count_all().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    let band_stats = db.stats_qso_per_band().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut by_band = HashMap::new();
    for (band, count) in band_stats {
        by_band.insert(band, count);
    }
    
    let mode_stats = db.stats_qso_per_mode().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut by_mode = HashMap::new();
    for (mode, count) in mode_stats {
        by_mode.insert(mode, count);
    }
    
    let (_tot, qsl_lotw, qsl_eqsl, _paper) = db.stats_qsl_summary().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    Ok(Json(StatsResponse {
        total,
        by_band,
        by_mode,
        qsl_lotw,
        qsl_eqsl,
    }))
}

/// GET /api/v1/cluster/spots?limit=N
/// Zwraca biezace spoty DX Cluster z bufora aplikacji.
async fn get_cluster_spots(
    State(state): State<ApiState>,
    Query(query): Query<ClusterQuery>,
) -> Json<Vec<serde_json::Value>> {
    let limit = query.limit.unwrap_or(50).min(200);
    let spots = state.cluster_spots.lock().unwrap_or_else(|e| e.into_inner());
    let result: Vec<serde_json::Value> = spots.iter().take(limit).map(|s| {
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
    }).collect();
    Json(result)
}
