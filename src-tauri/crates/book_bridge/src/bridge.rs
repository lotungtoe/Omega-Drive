use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    Router,
    http::StatusCode,
    routing::get,
};

use omega_drive_gateway::{
    provider::file_repository::FileRepository,
    provider::stream::StreamRegistry,
    provider::storage::PartMetadata,
    core::engine_context::EngineContext,
    player::cache::ByteCache,
    player::singleflight::PartSingleFlight,
};

use crate::formats::epub;
use crate::reader::ZipReader;

// ── Config ──────────────────────────────────────────────

#[derive(Clone)]
pub struct BookBridgeConfig {
    pub base_dir: std::path::PathBuf,
    pub file_repo: Arc<dyn FileRepository>,
    pub stream_registry: Arc<StreamRegistry>,
    pub engine: EngineContext,
    pub port: u16,
    pub byte_cache: Arc<dyn ByteCache>,
    pub singleflight: Arc<dyn PartSingleFlight>,
}

// ── Session manager ─────────────────────────────────────

struct BookSession {
    reader: Option<Arc<ZipReader>>,
    error: Option<String>,
}

pub struct BookManager {
    sessions: Arc<tokio::sync::Mutex<HashMap<i64, BookSession>>>,
}

impl BookManager {
    pub fn new() -> Self {
        Self { sessions: Arc::new(tokio::sync::Mutex::new(HashMap::new())) }
    }
}

// ── Bridge state (axum) ─────────────────────────────────

#[derive(Clone)]
pub struct BridgeState {
    pub cfg: Arc<BookBridgeConfig>,
    pub mgr: Arc<BookManager>,
}

// ── Public entry point ──────────────────────────────────

pub async fn start_book_bridge(config: BookBridgeConfig, manager: Arc<BookManager>) -> Result<u16, String> {
    let port = config.port;
    let state = BridgeState {
        cfg: Arc::new(config),
        mgr: manager,
    };

    let app = Router::new()
        .route("/book/:file_id/spine", get(epub::handle_spine))
        .route("/book/:file_id/nav", get(epub::handle_nav))
        .route("/book/:file_id/chapter/:chapter_index", get(epub::handle_chapter))
        .route("/book/:file_id/res/*path", get(epub::handle_resource))
        .layer(tower_http::cors::CorsLayer::permissive())
        .with_state(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await
        .map_err(|e| format!("book bridge bind: {e}"))?;
    let actual_port = listener.local_addr().map(|a| a.port()).unwrap_or(port);

    tracing::info!("book bridge ready http://127.0.0.1:{}", actual_port);
    tokio::spawn(async { axum::serve(listener, app).await.unwrap() });
    Ok(actual_port)
}

// ── Session helpers ─────────────────────────────────────

pub async fn get_or_start_session(
    file_id: i64,
    state: &BridgeState,
) -> Result<Arc<ZipReader>, (StatusCode, String)> {
    {
        let mut sessions = state.mgr.sessions.lock().await;
        match sessions.get(&file_id) {
            Some(BookSession { reader: Some(r), .. }) => return Ok(Arc::clone(r)),
            Some(BookSession { error: Some(e), .. }) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.clone())),
            Some(_) => {
                return Err((StatusCode::ACCEPTED, "loading".into()));
            }
            None => {
                sessions.insert(file_id, BookSession { reader: None, error: None });
            }
        }
    }

    let parts: Vec<PartMetadata> = state.cfg.file_repo.get_parts_for_file(file_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db: {e}")))?;

    let registry = Arc::clone(&state.cfg.stream_registry);
    let byte_cache = state.cfg.byte_cache.clone();
    let singleflight = state.cfg.singleflight.clone();
    let mgr = Arc::clone(&state.mgr);

    tokio::spawn(async move {
        let result = ZipReader::open(
            file_id, parts, registry,
            byte_cache, singleflight,
        ).await;
        let mut sessions = mgr.sessions.lock().await;
        if let Some(s) = sessions.get_mut(&file_id) {
            match result {
                Ok(reader) => s.reader = Some(Arc::new(reader)),
                Err(e) => s.error = Some(e),
            }
        }
    });

    Err((StatusCode::ACCEPTED, "loading".into()))
}

pub async fn wait_reader(file_id: i64, mgr: &BookManager) -> Result<Arc<ZipReader>, (StatusCode, String)> {
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(5 * 60);
    loop {
        if start.elapsed() > timeout {
            return Err((StatusCode::GATEWAY_TIMEOUT, "timeout".into()));
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
        let sessions = mgr.sessions.lock().await;
        if let Some(s) = sessions.get(&file_id) {
            if let Some(ref e) = s.error {
                return Err((StatusCode::INTERNAL_SERVER_ERROR, e.clone()));
            }
            if let Some(ref r) = s.reader {
                return Ok(Arc::clone(r));
            }
        }
    }
}
