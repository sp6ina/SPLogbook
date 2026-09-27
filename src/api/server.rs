use axum::{
    extract::{Path, Query, State},
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    http::{StatusCode, Method, header},
    response::Response,
    routing::get,
    Json, Router, middleware,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::collections::HashMap;

use crate::core::events::EventBus;
use crate::core::qso::QsoRecord;

use crate::cluster::telnet::DxSpot;

#[derive(Clone)]
pub struct ApiState {
    db: Arc<Mutex<crate::core::database::LogDatabase>>,
    callsign: String,
    start_time: Instant,
    pub cluster_spots: Arc<Mutex<Vec<DxSpot>>>,
    /// Centralna magistrala zdarzeń do przesyłania na żywo przez WebSocket.
    pub events: EventBus,
    api_key: String,
}

/// Generuje losowy klucz uwierzytelniający dla lokalnego serwera REST API.
/// Nie zależy od zewnętrznych bibliotek RNG: miesza entropię z kilku niezależnie
/// zainicjalizowanych `RandomState` (SipHash), które na większości platform
/// same czerpią losowość z systemowego generatora (getrandom/CryptGenRandom).
pub fn generate_api_key() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let mut key = String::with_capacity(32);
    for _ in 0..4 {
        let h = RandomState::new().build_hasher().finish();
        key.push_str(&format!("{:016x}", h));
    }
    key
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
    const HEADERS: HeaderValue = HeaderValue::from_static("Content-Type, Authorization, X-Api-Key");

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

/// Wymaga poprawnego nagłówka `X-Api-Key` dla wszystkich żądań poza publicznym
/// `/api/v1/status` (który nie ujawnia danych QSO) i preflightem CORS (OPTIONS).
/// Zapobiega to odczytowi/zapisowi dziennika przez dowolną stronę WWW lub proces
/// lokalny, który mógłby wykorzystać otwarte CORS (`*`) do wysyłania żądań do
/// localhost (atak typu "localhost drive-by" / DNS rebinding).
async fn auth_middleware(
    State(state): State<ApiState>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<Response, StatusCode> {
    if req.method() == Method::OPTIONS || req.uri().path() == "/api/v1/status" {
        return Ok(next.run(req).await);
    }

    let provided = req.headers().get("x-api-key").and_then(|v| v.to_str().ok());
    match provided {
        Some(key) if !state.api_key.is_empty() && key == state.api_key => Ok(next.run(req).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

pub async fn start_api_server(
    db: Arc<Mutex<crate::core::database::LogDatabase>>,
    callsign: String,
    port: u16,
    cluster_spots: Arc<Mutex<Vec<DxSpot>>>,
    api_key: String,
    events: EventBus,
) {
    let state = ApiState {
        db,
        callsign,
        start_time: Instant::now(),
        cluster_spots,
        events,
        api_key,
    };

    let app = Router::new()
        .route("/api/v1/status", get(get_status))
        .route("/api/v1/qsos", get(get_qsos).post(post_qso))
        .route("/api/v1/qsos/:id", get(get_qso_by_id))
        .route("/api/v1/stats", get(get_stats))
        .route("/api/v1/cluster/spots", get(get_cluster_spots))
        .route("/api/v1/ws", get(ws_handler))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
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
    eprintln!("[REST API] Wymagany naglowek uwierzytelniajacy X-Api-Key (patrz Narzedzia -> REST API).");
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
                        let text = match serde_json::to_string(&ev) {
                            Ok(t) => t,
                            Err(_) => continue,
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
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(payload))) => {
                        if sender.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) | Some(Ok(Message::Text(_))) | Some(Ok(Message::Binary(_))) => {}
                    Some(Err(_)) => break,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
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
}
