use axum::{
    extract::{ws, Path, State, WebSocketUpgrade},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;

use crate::config::Config;
use crate::viewer_assets::ViewerAssets;

pub type FrameSender = broadcast::Sender<String>;

#[derive(Clone)]
pub struct ViewerState {
    pub config: Arc<Config>,
    pub frame_tx: FrameSender,
    pub frame_count: Arc<std::sync::atomic::AtomicUsize>,
}

pub fn router(state: ViewerState) -> Router {
    // CORS: localhost only — recordings must not be accessible from external origins
    let cors = CorsLayer::new()
        .allow_origin([
            "http://localhost".parse().unwrap(),
            format!("http://localhost:{}", state.config.viewer.port)
                .parse()
                .unwrap(),
            format!("http://127.0.0.1:{}", state.config.viewer.port)
                .parse()
                .unwrap(),
        ])
        .allow_methods([axum::http::Method::GET])
        .allow_headers(tower_http::cors::Any);

    Router::new()
        .route("/", get(serve_index))
        .route("/assets/{*path}", get(serve_asset))
        .route("/api/runs", get(list_runs))
        .route("/api/runs/{name}", get(get_run))
        .route("/api/runs/{name}/frames", get(get_run_frames))
        .route("/api/status", get(get_status))
        .route("/ws/live", get(ws_handler))
        .layer(cors)
        .with_state(state)
}

async fn serve_index() -> impl IntoResponse {
    serve_embedded_file("index.html")
}

async fn serve_asset(Path(path): Path<String>) -> impl IntoResponse {
    serve_embedded_file(&format!("assets/{}", path))
}

fn serve_embedded_file(path: &str) -> Response {
    match ViewerAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, mime.as_ref().parse().unwrap());
            headers.insert(header::CACHE_CONTROL, "public, max-age=31536000".parse().unwrap());
            (StatusCode::OK, headers, content.data.into_owned()).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn list_runs() -> impl IntoResponse {
    let runs_dir = Config::runs_dir();
    let mut runs = vec![];
    if let Ok(entries) = std::fs::read_dir(&runs_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "ghostline").unwrap_or(false) {
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                let size = path.metadata().map(|m| m.len()).unwrap_or(0);
                runs.push(json!({"name": name, "size": size}));
            }
        }
    }
    Json(runs)
}

/// Sanitize a run name: reject path traversal, enforce .ghostline extension.
fn sanitize_run_name(name: &str) -> Option<&str> {
    // Must end with .ghostline
    if !name.ends_with(".ghostline") {
        return None;
    }
    // Must be a plain filename — no directory separators or traversal sequences
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return None;
    }
    // Extra: must not start with a dot (hidden files)
    if name.starts_with('.') {
        return None;
    }
    Some(name)
}

async fn get_run(Path(name): Path<String>) -> impl IntoResponse {
    let safe_name = match sanitize_run_name(&name) {
        Some(n) => n,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };
    let path = Config::runs_dir().join(safe_name);
    match std::fs::read(&path) {
        Ok(data) => {
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "application/octet-stream".parse().unwrap());
            (StatusCode::OK, headers, data).into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn get_run_frames(Path(name): Path<String>) -> impl IntoResponse {
    use ghostline_core::GhostlineReader;

    let safe_name = match sanitize_run_name(&name) {
        Some(n) => n,
        None => return (StatusCode::BAD_REQUEST, Json(json!([]))).into_response(),
    };
    let path = Config::runs_dir().join(safe_name);
    let mut reader = match GhostlineReader::open(path.to_str().unwrap_or("")) {
        Ok(r) => r,
        Err(_) => return (StatusCode::NOT_FOUND, Json(json!([]))).into_response(),
    };

    let mut frames = vec![];
    for i in 0..reader.frame_count() {
        if let Ok(frame) = reader.get_frame(i) {
            frames.push(json!({
                "index": i,
                "timestamp": frame.timestamp,
                "latency_ms": frame.latency_ms,
                "request_size": frame.request_bytes.len(),
                "response_size": frame.response_bytes.len(),
            }));
        }
    }
    Json(frames).into_response()
}

async fn get_status(State(state): State<ViewerState>) -> impl IntoResponse {
    let count = state.frame_count.load(std::sync::atomic::Ordering::Relaxed);
    Json(json!({
        "status": "ok",
        "frame_count": count,
        "proxy_port": state.config.proxy.port,
        "viewer_port": state.config.viewer.port,
    }))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<ViewerState>,
) -> impl IntoResponse {
    let mut rx = state.frame_tx.subscribe();
    ws.on_upgrade(move |mut socket| async move {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if socket
                        .send(ws::Message::Text(msg.into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => break,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
            }
        }
    })
}

pub async fn start(
    config: Arc<Config>,
    frame_tx: FrameSender,
    frame_count: Arc<std::sync::atomic::AtomicUsize>,
) -> anyhow::Result<()> {
    let port = config.viewer.port;
    let state = ViewerState {
        config,
        frame_tx,
        frame_count,
    };
    let app = router(state);
    // Bind to localhost only — viewer must not be exposed on the network
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    eprintln!(" ✓ Viewer serving on  http://localhost:{}", port);
    axum::serve(listener, app).await?;
    Ok(())
}
