use crate::PlayerContext;
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::sync::OnceLock;
use tokio::time::Duration;
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};

use crate::nativeplayer::{
    bridge_current_mpv_status, bridge_mpv_play_pause, bridge_mpv_seek, bridge_mpv_set_speed,
    bridge_mpv_set_volume, bridge_mpv_shutdown, bridge_mpv_toggle_fullscreen,
    bridge_open_native_player, BridgeOpenPlayerRequest, BridgeSeekRequest, BridgeSpeedRequest,
    BridgeVolumeRequest, MpvSessionType, MpvStatus,
};



struct ByteRangeTracker {
    last_start: u64,
    last_end: u64,
    last_detection: std::time::Instant,
}

static BYTE_RANGE_TRACKER: OnceLock<Mutex<HashMap<i64, ByteRangeTracker>>> = OnceLock::new();

/// Generation counter for `/raw/:file_id` requests.
/// Each new request bumps the generation for that file_id.
/// Old stream tasks check this at each part boundary and exit if stale.
pub(crate) static RAW_STREAM_GENERATION: OnceLock<Mutex<HashMap<i64, u64>>> = OnceLock::new();
pub(crate) static T1_MARK: OnceLock<Mutex<HashMap<i64, std::time::Instant>>> = OnceLock::new();

/// Determine Content-Type based on file extension (Video & Audio)
fn guess_media_content_type(filename: &str) -> &'static str {
    let lower = filename.to_ascii_lowercase();

    // Video types
    if lower.ends_with(".mp4") || lower.ends_with(".m4v") {
        "video/mp4"
    } else if lower.ends_with(".mkv") {
        "video/x-matroska"
    } else if lower.ends_with(".webm") {
        "video/webm"
    } else if lower.ends_with(".mov") {
        "video/quicktime"
    } else if lower.ends_with(".avi") {
        "video/x-msvideo"
    }
    // Audio types
    else if lower.ends_with(".mp3") {
        "audio/mpeg"
    } else if lower.ends_with(".wav") {
        "audio/wav"
    } else if lower.ends_with(".flac") {
        "audio/flac"
    } else if lower.ends_with(".ogg") {
        "audio/ogg"
    } else if lower.ends_with(".m4a") {
        "audio/mp4"
    } else if lower.ends_with(".aac") {
        "audio/aac"
    } else if lower.ends_with(".opus") {
        "audio/opus"
    } else {
        "application/octet-stream"
    }
}

#[derive(serde::Deserialize)]
struct MpvQuery {
    #[serde(rename = "type")]
    session_type: Option<MpvSessionType>,
}

/// Start HTTP Bridge, auto-probe port if the original port is occupied.
pub async fn start_bridge(st: PlayerContext) -> Result<u16, String> {
    let base_port = st.bridge_port.load(std::sync::atomic::Ordering::Relaxed);
    let max_probe = 100u16;
    crate::debug::debug_init(&st.base_dir);

    let app = Router::new()
        .route("/player/open", post(handle_player_open))
        .route("/player/status", get(handle_player_status))
        .route("/player/play-pause", post(handle_player_play_pause))
        .route("/player/seek", post(handle_player_seek))
        .route("/player/volume", post(handle_player_volume))
        .route("/player/speed", post(handle_player_speed))
        .route("/player/fullscreen", post(handle_player_fullscreen))
        .route("/player/shutdown", post(handle_player_shutdown))
        .route("/player/heartbeat", post(handle_player_heartbeat))
        .route("/raw/:file_id", get(handle_raw_file))
        .layer(TraceLayer::new_for_http())
        .layer(tower_http::cors::CorsLayer::permissive())
        .with_state(st.clone());

    for attempt in 0..max_probe {
        let port = base_port + attempt;
        match tokio::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], port))).await {
            Ok(listener) => {
                let bound_port = listener.local_addr().map(|a| a.port()).unwrap_or(port);
                st.bridge_port.store(bound_port, std::sync::atomic::Ordering::Relaxed);
                tokio::spawn(async move {
                    if let Err(e) = axum::serve(listener, app).await {
                        error!("Bridge server error: {}", e);
                    }
                });
                let working_ip = crate::infrastructure::pick_working_ip();
                debug_log!("start", "HTTP Bridge listening on :{} (working_ip={})", bound_port, working_ip);
                info!("HTTP Bridge listening on :{} (working_ip={})", bound_port, working_ip);
                return Ok(bound_port);
            }
            Err(_e) if attempt < max_probe - 1 => {
                warn!("Port {} taken, trying next", port);
                continue;
            }
            Err(e) => {
                return Err(format!(
                    "No free port in range {}-{}: {}",
                    base_port,
                    base_port + max_probe - 1,
                    e
                ));
            }
        }
    }
    unreachable!()
}

fn update_bridge_activity(st: &PlayerContext) {
    let now = Utc::now().timestamp() as u64;
    st.ui_last_heartbeat.store(now, Ordering::SeqCst);
}

async fn handle_player_heartbeat(State(st): State<PlayerContext>) -> impl IntoResponse {
    update_bridge_activity(&st);
    StatusCode::NO_CONTENT
}

async fn handle_player_open(
    State(st): State<PlayerContext>,
    Json(request): Json<BridgeOpenPlayerRequest>,
) -> impl IntoResponse {
    match bridge_open_native_player(&st, request).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err).into_response(),
    }
}

async fn handle_player_status(
    State(st): State<PlayerContext>,
    Query(params): Query<MpvQuery>,
) -> impl IntoResponse {
    match bridge_current_mpv_status(&st, params.session_type.unwrap_or(MpvSessionType::Video)).await
    {
        Ok(status) => Json::<MpvStatus>(status).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err).into_response(),
    }
}

async fn handle_player_shutdown(
    State(st): State<PlayerContext>,
    Query(params): Query<MpvQuery>,
) -> impl IntoResponse {
    match bridge_mpv_shutdown(&st, params.session_type.unwrap_or(MpvSessionType::Video)).await {
        Ok(status) => Json::<MpvStatus>(status).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err).into_response(),
    }
}

async fn handle_player_play_pause(
    State(st): State<PlayerContext>,
    Query(params): Query<MpvQuery>,
) -> impl IntoResponse {
    match bridge_mpv_play_pause(&st, params.session_type.unwrap_or(MpvSessionType::Video)).await {
        Ok(status) => Json::<MpvStatus>(status).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err).into_response(),
    }
}

async fn handle_player_seek(
    State(st): State<PlayerContext>,
    Query(params): Query<MpvQuery>,
    Json(request): Json<BridgeSeekRequest>,
) -> impl IntoResponse {
    match bridge_mpv_seek(
        &st,
        request.position,
        params.session_type.unwrap_or(MpvSessionType::Video),
    )
    .await
    {
        Ok(status) => Json::<MpvStatus>(status).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err).into_response(),
    }
}

async fn handle_player_volume(
    State(st): State<PlayerContext>,
    Query(params): Query<MpvQuery>,
    Json(request): Json<BridgeVolumeRequest>,
) -> impl IntoResponse {
    match bridge_mpv_set_volume(
        &st,
        request.volume,
        params.session_type.unwrap_or(MpvSessionType::Video),
    )
    .await
    {
        Ok(status) => Json::<MpvStatus>(status).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err).into_response(),
    }
}

async fn handle_player_speed(
    State(st): State<PlayerContext>,
    Query(params): Query<MpvQuery>,
    Json(request): Json<BridgeSpeedRequest>,
) -> impl IntoResponse {
    match bridge_mpv_set_speed(
        &st,
        request.speed,
        params.session_type.unwrap_or(MpvSessionType::Video),
    )
    .await
    {
        Ok(status) => Json::<MpvStatus>(status).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err).into_response(),
    }
}

async fn handle_player_fullscreen(
    State(st): State<PlayerContext>,
    Query(params): Query<MpvQuery>,
) -> impl IntoResponse {
    match bridge_mpv_toggle_fullscreen(&st, params.session_type.unwrap_or(MpvSessionType::Video))
        .await
    {
        Ok(status) => Json::<MpvStatus>(status).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err).into_response(),
    }
}

async fn handle_raw_file(
    Path(file_id): Path<i64>,
    State(st): State<PlayerContext>,
    headers: HeaderMap,
) -> impl IntoResponse {
    update_bridge_activity(&st);
    debug_log!("seek", "bridge_raw: file={}", file_id);
    // Ensure file is ready for playback (check cache, decrypt, etc.)
    let (file_size, chunk_size, content_type, duration_sec) =
        match crate::ensure_video_playback_ready(&st, file_id).await {
            Ok(file) => {
                let mut chunk_size = st.cfg.general.chunk_bytes;
                if let Some(first) = st.player_runtime.get_first_original_part(file_id).await {
                    let logical_size = first.size.max(0) as u64;
                    if logical_size > 0 {
                        chunk_size = logical_size;
                    }
                } else if let Ok(parts) = st.file_repo.get_original_parts_for_file(file_id).await {
                    st.player_runtime
                        .cache_original_parts(file_id, parts.clone())
                        .await;
                    if let Some(first) = parts.into_iter().find(|p| p.part_index == 1) {
                        let logical_size = first.size.max(0) as u64;
                        if logical_size > 0 {
                            chunk_size = logical_size;
                        }
                    }
                }
                (
                    file.size,
                    chunk_size,
                    guess_media_content_type(&file.filename),
                    file.duration_sec,
                )
            }
            Err(e) => return (StatusCode::CONFLICT, e).into_response(),
        };

    let start_time = std::time::Instant::now();
    let range_header = headers.get(header::RANGE).and_then(|v| v.to_str().ok());
    let ua = headers.get("user-agent").and_then(|v| v.to_str().ok()).unwrap_or("none");
    debug_log!("http", "bridge_raw: file={} range={:?} ua={} size={}",
        file_id, range_header, ua, file_size);

    let result_data = match parse_range(range_header, file_size) {
        Some((start, end)) => {
            T1_MARK.get_or_init(|| Mutex::new(HashMap::new()))
                .lock().expect("Mutex poisoned")
                .insert(file_id, std::time::Instant::now());
            let content_length = end - start + 1;

            // Byte-range heuristic: detect seek from non-sequential byte jump
            let detected_pts = {
                let tracker = BYTE_RANGE_TRACKER.get_or_init(|| Mutex::new(HashMap::new()));
                let mut map = tracker.lock().expect("Mutex poisoned");
                let now = std::time::Instant::now();
                if let Some(t) = map.get(&file_id) {
                    let prev_detection = t.last_detection;
                    let gap_threshold = chunk_size.max(1024 * 1024);
                    let is_backward =
                        start < t.last_start && t.last_start.saturating_sub(start) > gap_threshold;
                    let is_forward = start > t.last_end.saturating_add(gap_threshold);
                    let is_moov_probe = is_backward
                        && t.last_start > (file_size as u64).saturating_sub(64 * 1024 * 1024)
                        && t.last_start.saturating_sub(start) as f64 > file_size as f64 * 0.5;
                    let result = if (is_backward || is_forward)
                        && !is_moov_probe
                        && prev_detection.elapsed() > std::time::Duration::from_millis(500)
                    {
                        let gap = if start > t.last_end {
                            start.saturating_sub(t.last_end)
                        } else {
                            t.last_end.saturating_sub(start)
                        };
                        debug_log!(
                            "seek",
                            "byte_range_heuristic: seek detected file={} gap={} start={} dur={:?}",
                            file_id,
                            gap,
                            start,
                            duration_sec
                        );
                        duration_sec
                            .filter(|d| *d > 0.0)
                            .map(|dur| (start as f64 / file_size as f64) * dur)
                    } else {
                        None
                    };
                    map.insert(
                        file_id,
                        ByteRangeTracker {
                            last_start: start,
                            last_end: end,
                            last_detection: if result.is_some() {
                                now
                            } else {
                                prev_detection
                            },
                        },
                    );
                    result
                } else {
                    map.insert(
                        file_id,
                        ByteRangeTracker {
                            last_start: start,
                            last_end: end,
                            last_detection: now,
                        },
                    );
                    None
                }
            };
            if let Some(estimated_pts) = detected_pts {
                st.player_runtime
                    .record_recent_seek_target(file_id, estimated_pts)
                    .await;
                debug_log!(
                    "seek",
                    "byte_range_heuristic: recorded file={} pts={:.3}s",
                    file_id,
                    estimated_pts
                );
            }

            tracing::debug!(
                "Bridge RAW [START]: file {} range {}-{} (Len: {})",
                file_id,
                start,
                end,
                content_length
            );

            let recent_seek_pts = st
                .player_runtime
                .peek_recent_seek_target(file_id, Duration::from_millis(2_000))
                .await;
            debug_log!(
                "seek",
                "bridge_raw: file={} range={}-{} recent_seek_pts={:?}",
                file_id,
                start,
                end,
                recent_seek_pts
            );
            // Bump stream generation to cancel stale tasks for this file_id
            let stream_gen = {
                let mut map = RAW_STREAM_GENERATION
                    .get_or_init(|| Mutex::new(HashMap::new()))
                    .lock()
                    .expect("Mutex poisoned");
                let g = map.entry(file_id).or_insert(0);
                *g += 1;
                *g
            };
            match crate::stream::stream_byte_range(st, file_id, start, end, stream_gen).await {
                Ok(stream) => {
                    let mut response = Response::new(Body::from_stream(stream));
                    *response.status_mut() = StatusCode::PARTIAL_CONTENT;
                    let content_range = match HeaderValue::from_str(&format!(
                        "bytes {}-{}/{}",
                        start, end, file_size
                    )) {
                        Ok(value) => value,
                        Err(e) => {
                            debug_log!("error", "bridge_raw: invalid CONTENT_RANGE header file={} err={}", file_id, e);
                            tracing::error!("Bridge RAW: invalid CONTENT_RANGE header for file {}: {}", file_id, e);
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Invalid content-range header",
                            )
                                .into_response();
                        }
                    };
                    response
                        .headers_mut()
                        .insert(header::CONTENT_RANGE, content_range);
                    response
                        .headers_mut()
                        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
                    response
                        .headers_mut()
                        .insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
                    // ponytail: no Content-Length — body is chunked (transfer-encoding),
                    // avoids libcurl AVERROR_EOF when MPV disconnects early to seek
                    response.into_response()
                }
                Err(e) => {
                    debug_log!("error", "bridge_raw partial fail: file={} err={}", file_id, e);
                    tracing::error!("Bridge RAW Error [PARTIAL]: file {} error: {}", file_id, e);
                    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
                }
            }
        }
        None => {
            // Case: no Range header, return file info so client knows we support Range
            tracing::debug!(
                "[Bridge] RAW [FULL]: file {}",
                file_id
            );
            // Return headers so FFmpeg knows it can use Range requests
            let mut response = Response::new(Body::empty());
            response
                .headers_mut()
                .insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
            response
                .headers_mut()
                .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
            let content_length_header = match HeaderValue::from_str(&file_size.to_string()) {
                Ok(value) => value,
                Err(e) => {
                    debug_log!("error", "bridge_raw: invalid CONTENT_LENGTH (full) file={} err={}", file_id, e);
                    tracing::error!("Bridge RAW: invalid CONTENT_LENGTH header for file {}: {}", file_id, e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Invalid content-length header",
                    )
                        .into_response();
                }
            };
            response
                .headers_mut()
                .insert(header::CONTENT_LENGTH, content_length_header);
            debug_log!("http", "bridge_raw: no_range file={} status=200 cl={} ct={} body=empty",
                file_id, file_size, content_type);
            response.into_response()
        }
    };

    let dur = start_time.elapsed().as_millis();
    if dur > 2000 {
        debug_log!("seek", "bridge_raw slow: file={} took={}ms", file_id, dur);
        tracing::warn!("Bridge RAW SLOW: file {} took {}ms!", file_id, dur);
    }
    result_data
}

fn parse_range(range: Option<&str>, file_size: i64) -> Option<(u64, u64)> {
    let range = range?;
    if !range.starts_with("bytes=") {
        return None;
    }
    let parts: Vec<&str> = range["bytes=".len()..].split('-').collect();
    if parts.len() != 2 {
        return None;
    }
    let file_size_u64 = file_size.max(0) as u64;
    if file_size_u64 == 0 {
        return None;
    }
    if parts[0].is_empty() {
        // Suffix range: bytes=-N (last N bytes)
        let suffix = parts[1].parse::<u64>().ok()?;
        if suffix == 0 {
            return None;
        }
        let end = file_size_u64 - 1;
        let start = file_size_u64.saturating_sub(suffix);
        return Some((start, end));
    }
    let start = parts[0].parse::<u64>().ok()?;
    let end = if parts[1].is_empty() {
        file_size_u64 - 1
    } else {
        parts[1].parse::<u64>().ok()?
    };
    if start > end {
        return None;
    }
    Some((start, std::cmp::min(end, file_size_u64 - 1)))
}



