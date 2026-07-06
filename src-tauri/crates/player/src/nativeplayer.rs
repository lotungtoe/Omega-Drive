use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, OnceLock,
    },
    thread,
    time::{Duration, Instant},
};


use tracing::{info, warn};

use crate::PlayerContext;
use omega_drive_gateway::provider::file_repository::FileRepository;
use omega_drive_gateway::provider::storage::PartMetadata;

const PLAYBACK_MIN_SAVE_POSITION_SECS: f64 = 10.0;
const RAW_SEEK_CACHE_SECS_CAP: u64 = 5;
const RAW_SEEK_DEMUXER_MAX_MB_CAP: u64 = 64;
const RAW_SEEK_READAHEAD_SECS_CAP: u64 = 5;
const SEEK_DETECTION_THRESHOLD_SECS: f64 = 5.0;
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MpvSessionType {
    Video,
    Audio,
}

struct MpvSessions {
    video: Arc<Mutex<Option<NativeMpvSession>>>,
    audio: Arc<Mutex<Option<NativeMpvSession>>>,
}

static NATIVE_MPV_SESSIONS: OnceLock<MpvSessions> = OnceLock::new();
static NEXT_NATIVE_MPV_SESSION_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct MpvStatus {
    pub alive: bool,
    pub paused: bool,
    pub position: f64,
    pub duration: f64,
    pub volume: f64,
    pub speed: f64,
    pub fullscreen: bool,
    pub title: String,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct BridgeOpenPlayerRequest {
    pub file_id: i64,
    pub title: String,
    pub start_position_sec: Option<f64>,
    pub session_type: Option<MpvSessionType>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct BridgeSeekRequest {
    pub position: f64,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct BridgeVolumeRequest {
    pub volume: f64,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct BridgeSpeedRequest {
    pub speed: f64,
}

impl Default for MpvStatus {
    fn default() -> Self {
        Self {
            alive: false,
            paused: true,
            position: 0.0,
            duration: 0.0,
            volume: 100.0,
            speed: 1.0,
            fullscreen: false,
            title: String::new(),
        }
    }
}

struct NativeMpvSession {
    id: u64,
    file_id: i64,
    window_label: String,
    title: String,
    status: MpvStatus,
    mpv: libmpv2::Mpv,
    pending_default_audio: Option<i64>,
    pending_audio_tracks: Vec<(String, String)>,
    audio_tracks_loaded: bool,
}

// SAFETY: libmpv's client API is documented as thread-safe, and this session is
// only accessed behind a process-wide Mutex so calls are serialized.
unsafe impl Send for NativeMpvSession {}

fn playback_completion_tail(duration_sec: f64) -> f64 {
    (duration_sec * 0.1).clamp(3.0, 20.0)
}

fn playback_should_clear(position_sec: f64, duration_sec: Option<f64>) -> bool {
    let Some(duration_sec) = duration_sec.filter(|value| value.is_finite() && *value > 0.0) else {
        return false;
    };

    position_sec >= duration_sec - playback_completion_tail(duration_sec)
}

fn normalize_start_position_sec(start_position_sec: Option<f64>) -> f64 {
    start_position_sec
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(0.0)
}

fn should_release_file_cache(active_file_id: Option<i64>, closing_file_id: i64) -> bool {
    !matches!(active_file_id, Some(current_file_id) if current_file_id == closing_file_id)
}

fn mpv_demuxer_max_bytes(max_mb: u64) -> u64 {
    max_mb.saturating_mul(1024).saturating_mul(1024)
}

fn current_session_file_id(session_type: MpvSessionType) -> Option<i64> {
    let guard = match mpv_session_store_by_type(session_type).lock() {
        Ok(guard) => guard,
        Err(err) => {
            warn!(
                "Native mpv session store ({:?}) poisoned while checking active file: {}",
                session_type, err
            );
            err.into_inner()
        }
    };

    guard.as_ref().map(|session| session.file_id)
}

fn release_closed_file_cache(
    state: &PlayerContext,
    active_file_id: Option<i64>,
    file_id: i64,
    handle: &tokio::runtime::Handle,
) {
    if should_release_file_cache(active_file_id, file_id) {
        state
            .player_runtime
            .sparse_cache
            .unpin_file(file_id);
        let player_runtime = Arc::clone(&state.player_runtime);
        handle.spawn(async move {
            player_runtime.clear_hot_file_state(file_id).await;
        });
    }
}

async fn warm_playback_runtime_state(
    state: &PlayerContext,
    file_id: i64,
    parts: Vec<PartMetadata>,
) {
    if parts.is_empty() {
        return;
    }

    state
        .player_runtime
        .cache_original_parts(file_id, parts.clone())
        .await;

    let mut by_provider: HashMap<String, Vec<PartMetadata>> = HashMap::new();
    for part in parts {
        by_provider
            .entry(part.platform.clone())
            .or_default()
            .push(part);
    }

    for (provider_id, provider_parts) in by_provider {
        let Some(gateway) = state.stream_registry.get(&provider_id) else {
            continue;
        };
        if let Err(err) = gateway.prepare_parts_for_playback(&provider_parts).await {
            warn!(
                "Playback runtime warmup failed for file {} provider {}: {}",
                file_id, provider_id, err
            );
        }
    }
}

fn configure_native_player_buffering(
    builder: &libmpv2::MpvInitializer,
    state: &PlayerContext,
    seek_sensitive: bool,
) -> Result<(), String> {
    let (cache_secs, demuxer_max_mb, readahead_secs) = effective_mpv_buffer_values(
        state.cfg.mpv_cache_secs,
        state.cfg.mpv_demuxer_max_mb,
        state.cfg.mpv_readahead_secs,
        seek_sensitive,
    );
    let cache_secs = cache_secs.to_string();
    let demuxer_max_bytes = mpv_demuxer_max_bytes(demuxer_max_mb).to_string();
    let readahead_secs = readahead_secs.to_string();

    builder
        .set_option("cache", "yes")
        .map_err(|err| format_mpv_error("Failed to enable mpv cache", err))?;
    builder
        .set_option("cache-secs", cache_secs.as_str())
        .map_err(|err| format_mpv_error("Failed to set mpv cache-secs", err))?;
    builder
        .set_option("demuxer-max-bytes", demuxer_max_bytes.as_str())
        .map_err(|err| format_mpv_error("Failed to set mpv demuxer-max-bytes", err))?;
    builder
        .set_option("demuxer-max-back-bytes", demuxer_max_bytes.as_str())
        .map_err(|err| format_mpv_error("Failed to set mpv demuxer-max-back-bytes", err))?;
    builder
        .set_option("demuxer-readahead-secs", readahead_secs.as_str())
        .map_err(|err| format_mpv_error("Failed to set mpv demuxer-readahead-secs", err))?;

    Ok(())
}

fn effective_mpv_buffer_values(
    cache_secs: u64,
    demuxer_max_mb: u64,
    readahead_secs: u64,
    seek_sensitive: bool,
) -> (u64, u64, u64) {
    if !seek_sensitive {
        return (cache_secs, demuxer_max_mb, readahead_secs);
    }

    (
        cache_secs.min(RAW_SEEK_CACHE_SECS_CAP),
        demuxer_max_mb.min(RAW_SEEK_DEMUXER_MAX_MB_CAP),
        readahead_secs.min(RAW_SEEK_READAHEAD_SECS_CAP),
    )
}

fn persist_playback_snapshot(
    file_repo: &Arc<dyn FileRepository>,
    file_id: i64,
    position_sec: f64,
    duration_sec: Option<f64>,
    handle: &tokio::runtime::Handle,
) {
    let should_clear = position_sec < PLAYBACK_MIN_SAVE_POSITION_SECS
        || playback_should_clear(position_sec, duration_sec);

    let file_repo = Arc::clone(file_repo);
    let _ = handle.block_on(async move {
        if should_clear {
            file_repo.clear_playback_history(file_id).await
        } else {
            file_repo.save_playback_history(file_id, position_sec, duration_sec, false).await
        }
    });
}

async fn persist_playback_snapshot_async(
    file_repo: &Arc<dyn FileRepository>,
    file_id: i64,
    position_sec: f64,
    duration_sec: Option<f64>,
) {
    let should_clear = position_sec < PLAYBACK_MIN_SAVE_POSITION_SECS
        || playback_should_clear(position_sec, duration_sec);

    let result = if should_clear {
        file_repo.clear_playback_history(file_id).await
    } else {
        file_repo.save_playback_history(file_id, position_sec, duration_sec, false).await
    };

    if let Err(err) = result {
        warn!(
            "Failed to persist native playback history for file {} at {:.2}s: {}",
            file_id, position_sec, err
        );
    }
}

fn mpv_sessions() -> &'static MpvSessions {
    NATIVE_MPV_SESSIONS.get_or_init(|| MpvSessions {
        video: Arc::new(Mutex::new(None)),
        audio: Arc::new(Mutex::new(None)),
    })
}

fn mpv_session_store_by_type(
    session_type: MpvSessionType,
) -> &'static Arc<Mutex<Option<NativeMpvSession>>> {
    let sessions = mpv_sessions();
    match session_type {
        MpvSessionType::Video => &sessions.video,
        MpvSessionType::Audio => &sessions.audio,
    }
}

fn native_player_runtime_error() -> String {
    "Native player runtime (mpv) is not ready.".to_string()
}

fn format_mpv_error(context: &str, err: impl std::fmt::Debug) -> String {
    format!("{context}: {err:?}")
}

fn format_mpv_script_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn resolve_custom_player_ui_script() -> Result<PathBuf, String> {
    let current_dir = std::env::current_dir()
        .map_err(|err| format!("Failed to read working directory for mpv Lua UI: {err}"))?;
    let mut candidates = vec![
        current_dir
            .join("src-tauri")
            .join("resources")
            .join("player")
            .join("player_ui.lua"),
        current_dir
            .join("resources")
            .join("player")
            .join("player_ui.lua"),
    ];

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            let exe_resource_path = exe_dir
                .join("resources")
                .join("player")
                .join("player_ui.lua");
            if !candidates.contains(&exe_resource_path) {
                candidates.push(exe_resource_path);
            }

            if let Some(app_dir) = exe_dir.parent() {
                let bundle_resource_path = app_dir
                    .join("Resources")
                    .join("resources")
                    .join("player")
                    .join("player_ui.lua");
                if !candidates.contains(&bundle_resource_path) {
                    candidates.push(bundle_resource_path);
                }
            }
        }
    }

    if let Some(path) = candidates.iter().find(|path| path.is_file()) {
        return Ok(path.clone());
    }

    let tried = candidates
        .iter()
        .map(|path| format!("'{}'", path.display()))
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!(
        "Failed to locate bundled mpv Lua UI script. Tried {tried}."
    ))
}

fn configure_keep_open_at_eof(builder: &libmpv2::MpvInitializer) -> Result<(), String> {
    builder
        .set_option("keep-open", "yes")
        .map_err(|err| format_mpv_error("Failed to keep mpv window open at EOF", err))?;

    Ok(())
}

fn configure_custom_player_ui(builder: &libmpv2::MpvInitializer) -> Result<(), String> {
    let script_path = resolve_custom_player_ui_script()?;
    let script_path = format_mpv_script_path(&script_path);

    builder
        .set_option("osc", false)
        .map_err(|err| format_mpv_error("Failed to disable built-in mpv OSC", err))?;
    builder
        .set_option("scripts", script_path.as_str())
        .map_err(|err| format_mpv_error("Failed to load custom mpv Lua UI script", err))?;
    configure_keep_open_at_eof(builder)?;
    builder
        .set_option("cursor-autohide", "1000")
        .map_err(|err| format_mpv_error("Failed to enable cursor autohide", err))?;

    Ok(())
}

fn emit_playback_activity(state: &PlayerContext, active: bool, window_label: &str) {
    let any_active = {
        let mut windows = match state.player_runtime.active_playback_windows.lock() {
            Ok(lock) => lock,
            Err(err) => {
                warn!("Failed to lock playback activity state: {}", err);
                return;
            }
        };

        if active {
            windows.insert(window_label.to_string());
        } else {
            windows.remove(window_label);
        }

        !windows.is_empty()
    };

    state.event_emitter.emit("playback-state-changed", serde_json::json!(any_active));
}

fn refresh_session_status(session: &mut NativeMpvSession, state: &PlayerContext) {
    if !session.status.alive {
        return;
    }

    let mut reached_eof = false;
    while let Some(Ok(event)) = session.mpv.wait_event(0.0) {
        match event {
            libmpv2::events::Event::EndFile(reason) => {
                if reason == libmpv2::mpv_end_file_reason::Eof as u32 {
                    reached_eof = true;
                } else if reason == libmpv2::mpv_end_file_reason::Error as u32 {
                    debug_log!("mpv", "EndFile(Error) for file {}: reason={}", session.file_id, reason);
                    session.status.alive = false;
                    break;
                } else {
                    debug_log!("mpv", "EndFile({}) for file {}: alive=false", reason, session.file_id);
                    session.status.alive = false;
                    break;
                }
            }
            libmpv2::events::Event::Shutdown => {
                session.status.alive = false;
                break;
            }
            libmpv2::events::Event::PropertyChange {
                reply_userdata: 6969,
                name: _name,
                change: libmpv2::events::PropertyData::Str(val),
            } => {
                if val.starts_with("subtitle") {
                    state.event_emitter.emit(
                        "open-attachment-picker",
                        serde_json::json!({ "file_id": session.file_id, "type": "subtitle" }),
                    );
                } else if val.starts_with("audio") {
                    state.event_emitter.emit(
                        "open-attachment-picker",
                        serde_json::json!({ "file_id": session.file_id, "type": "audio" }),
                    );
                }
            }
            libmpv2::events::Event::PropertyChange {
                reply_userdata: 6970,
                name: _name,
                change: libmpv2::events::PropertyData::Int64(_),
            } => {
                if let Ok(url) = session.mpv.get_property::<String>("current-tracks/audio/external-filename") {
                    if let Some(file_id_str) = url.split("/raw/").nth(1) {
                        if let Ok(audio_file_id) = file_id_str.trim_end().parse::<i64>() {
                            session.pending_default_audio = Some(audio_file_id);
                        } else {
                            session.pending_default_audio = Some(-1);
                        }
                    } else {
                        session.pending_default_audio = Some(-1);
                    }
                } else {
                    session.pending_default_audio = Some(-1);
                }
            }
            libmpv2::events::Event::PropertyChange {
                reply_userdata: 6971,
                name: _name,
                change: libmpv2::events::PropertyData::Double(val),
            } if val > 0.0 && !session.audio_tracks_loaded => {
                let tracks = std::mem::take(&mut session.pending_audio_tracks);
                for (url, flag) in &tracks {
                    if let Err(e) = session.mpv.command("audio-add", &[url.as_str(), flag.as_str()]) {
                        tracing::error!("Failed to add audio track {}: {}", url, e);
                    } else {
                        info!("Loaded deferred audio track: {}", url);
                    }
                }
                session.audio_tracks_loaded = true;
                let _ = session.mpv.unobserve_property(6971);
            }
            _ => {
                debug_log!("mpv_event", "unhandled event for file {}: {:?}", session.file_id, std::mem::discriminant(&event));
            }
        }
    }

    if !session.status.alive {
        return;
    }

    session.status.paused = session
        .mpv
        .get_property::<bool>("pause")
        .unwrap_or(session.status.paused);

    let position = session
        .mpv
        .get_property::<f64>("time-pos")
        .unwrap_or(session.status.position);
    if position.is_finite() && position >= 0.0 {
        session.status.position = position;
    }

    let duration = session
        .mpv
        .get_property::<f64>("duration")
        .unwrap_or(session.status.duration);
    if duration.is_finite() && duration > 0.0 {
        session.status.duration = duration;
    }

    let volume = session
        .mpv
        .get_property::<f64>("volume")
        .unwrap_or(session.status.volume);
    session.status.volume = volume.clamp(0.0, 100.0);

    let speed = session
        .mpv
        .get_property::<f64>("speed")
        .unwrap_or(session.status.speed);
    session.status.speed = speed.clamp(0.25, 4.0);

    session.status.fullscreen = session
        .mpv
        .get_property::<bool>("fullscreen")
        .unwrap_or(session.status.fullscreen);

    let eof_reached = session
        .mpv
        .get_property::<bool>("eof-reached")
        .unwrap_or(false);
    if reached_eof || eof_reached {
        session.status.paused = true;
        if session.status.duration > 0.0 {
            session.status.position = session.status.duration;
        }
    }

    session.status.title = session.title.clone();
}

fn save_video_progress(session: &NativeMpvSession) -> (i64, f64, Option<f64>) {
    (
        session.file_id,
        session.status.position,
        (session.status.duration > 0.0).then_some(session.status.duration),
    )
}

fn take_session_for_teardown(
    session_slot: &mut Option<NativeMpvSession>,
    state: &PlayerContext,
) -> Option<NativeMpvSession> {
    let mut session = session_slot.take()?;
    refresh_session_status(&mut session, state);
    Some(session)
}

fn shutdown_native_session(session: &mut NativeMpvSession, state: &PlayerContext) {
    let _ = session.mpv.command("stop", &[]);
    let _ = session.mpv.command("playlist-clear", &[]);
    let _ = session.mpv.command("quit", &[]);
    emit_playback_activity(state, false, &session.window_label);
}

fn teardown_session(
    session_slot: &mut Option<NativeMpvSession>,
    state: &PlayerContext,
    handle: &tokio::runtime::Handle,
) {
    let Some(mut session) = take_session_for_teardown(session_slot, state) else {
        return;
    };

    let (file_id, position_sec, duration_sec) = save_video_progress(&session);
    shutdown_native_session(&mut session, state);
    drop(session);
    release_closed_file_cache(state, None, file_id, handle);
    persist_playback_snapshot(&state.file_repo, file_id, position_sec, duration_sec, handle);
}

async fn teardown_taken_session_async(
    mut session: NativeMpvSession,
    state: &PlayerContext,
    session_type: MpvSessionType,
) {
    let handle = tokio::runtime::Handle::current();
    let (file_id, position_sec, duration_sec) = save_video_progress(&session);
    shutdown_native_session(&mut session, state);
    drop(session);
    let active_file_id = current_session_file_id(session_type);
    release_closed_file_cache(state, active_file_id, file_id, &handle);
    persist_playback_snapshot_async(&state.file_repo, file_id, position_sec, duration_sec).await;
}

async fn current_mpv_status(
    state: &PlayerContext,
    session_type: MpvSessionType,
) -> Result<MpvStatus, String> {
    let stale_session = {
        let mut guard = mpv_session_store_by_type(session_type)
            .lock()
            .map_err(|_| format!("Failed to lock native mpv {:?} session", session_type))?;

        if guard.is_none() {
            return Ok(MpvStatus::default());
        }

        {
            let session = guard.as_mut().expect("checked above");
            refresh_session_status(session, state);
        }

        let is_alive = guard
            .as_ref()
            .map(|session| session.status.alive)
            .unwrap_or(false);
        if !is_alive {
            take_session_for_teardown(&mut *guard, state)
        } else {
            return Ok(guard
                .as_ref()
                .map(|session| session.status.clone())
                .unwrap_or_default());
        }
    };

    if let Some(session) = stale_session {
        teardown_taken_session_async(session, state, session_type).await;
    }

    Ok(MpvStatus::default())
}

async fn mutate_mpv_session(
    state: &PlayerContext,
    session_type: MpvSessionType,
    mutator: impl FnOnce(&mut NativeMpvSession) -> Result<(), String>,
) -> Result<MpvStatus, String> {
    let stale_session = {
        let mut guard = mpv_session_store_by_type(session_type)
            .lock()
            .map_err(|_| format!("Failed to lock native mpv {:?} session", session_type))?;

        if guard.is_none() {
            return Ok(MpvStatus::default());
        }

        {
            let session = guard.as_mut().expect("checked above");
            refresh_session_status(session, state);
        }

        let is_alive = guard
            .as_ref()
            .map(|session| session.status.alive)
            .unwrap_or(false);
        if !is_alive {
            take_session_for_teardown(&mut *guard, state)
        } else {
            {
                let session = guard.as_mut().expect("checked above");
                mutator(session)?;
                refresh_session_status(session, state);
            }

            let is_alive = guard
                .as_ref()
                .map(|session| session.status.alive)
                .unwrap_or(false);
            if !is_alive {
                take_session_for_teardown(&mut *guard, state)
            } else {
                return Ok(guard
                    .as_ref()
                    .map(|session| session.status.clone())
                    .unwrap_or_default());
            }
        }
    };

    if let Some(session) = stale_session {
        teardown_taken_session_async(session, state, session_type).await;
    }

    Ok(MpvStatus::default())
}

fn spawn_session_monitor(
    state: PlayerContext,
    session_id: u64,
    session_type: MpvSessionType,
    handle: tokio::runtime::Handle,
) {
    let session_store = Arc::clone(mpv_session_store_by_type(session_type));

    thread::spawn(move || {
        let mut last_position: f64 = f64::NEG_INFINITY;
        let mut last_seek_detected: Option<Instant> = None;
        let mut stall_counter: u32 = 0;
        let monitor_start = Instant::now();

        loop {
            let mut should_break = false;
            let mut detected_seek: Option<(i64, f64)> = None;

            let mut guard = match session_store.lock() {
                Ok(guard) => guard,
                Err(err) => {
                    warn!("Failed to lock native mpv session monitor: {}", err);
                    break;
                }
            };

            let session_alive;
            let session_file_id;

            {
                let Some(session) = guard.as_mut() else {
                    break;
                };

                if session.id != session_id {
                    break;
                }

                refresh_session_status(session, &state);
                session_alive = session.status.alive;
                session_file_id = session.file_id;

                if let Some(audio_file_id) = session.pending_default_audio.take() {
                    let vid = session.file_id;
                    let st = state.clone();
                    handle.spawn(async move {
                        if audio_file_id > 0 {
                            if let Ok(Some(vm)) = st.file_repo.get_video_file(vid).await {
                                let mut audio_ids: Vec<i64> = vm.audio.as_deref()
                                    .and_then(|j| serde_json::from_str(j).ok())
                                    .unwrap_or_default();
                                if !audio_ids.contains(&audio_file_id) {
                                    audio_ids.push(audio_file_id);
                                }
                                let aj = serde_json::to_string(&audio_ids).unwrap_or_else(|_| "[]".to_string());
                                let _ = st.file_repo.update_video_audio(vid, &aj, Some(audio_file_id)).await;
                            }
                        } else if let Ok(Some(vm)) = st.file_repo.get_video_file(vid).await {
                            let aj = vm.audio.unwrap_or_else(|| "[]".to_string());
                            let _ = st.file_repo.update_video_audio(vid, &aj, None).await;
                        }
                    });
                }

                // Only check stall/seek when session is alive AND not paused
                // (paused state naturally has no position change)
                if session_alive && !session.status.paused {
                    let position = session.status.position;
                    if position.is_finite() && position >= 0.0 {
                        if last_position.is_finite() && last_position >= 0.0
                            && monitor_start.elapsed() > Duration::from_secs(3)
                        {
                            // Stall detection: position unchanged for 5+ seconds WHILE PLAYING
                            if (position - last_position).abs() < 0.001 {
                                stall_counter += 1;

                                // Log every 5 seconds (20 ticks) for debugging
                                if stall_counter % 20 == 0 {
                                    warn!(
                                        "[stall] monitor: file={} pos={:.3}s stalled for {:.1}s (alive={} paused={} duration={:.1}s)",
                                        session_file_id, position, stall_counter as f64 * 0.25,
                                        session.status.alive, session.status.paused, session.status.duration
                                    );
                                }
                            } else {
                                if stall_counter >= 20 {
                                    info!(
                                        "[stall] monitor: file={} recovered pos={:.3}s -> {:.3}s after {:.1}s stall",
                                        session_file_id, last_position, position, stall_counter as f64 * 0.25
                                    );
                                }
                                stall_counter = 0;
                            }

                            let delta = (position - last_position).abs();
                            if delta > SEEK_DETECTION_THRESHOLD_SECS {
                                let debounced = match last_seek_detected {
                                    Some(t) => t.elapsed() > Duration::from_millis(500),
                                    None => true,
                                };
                                if debounced {
                                    info!(
                                        "[seek] monitor: file={} detected jump {:.3}s -> {:.3}s (delta={:.3}s)",
                                        session_file_id, last_position, position, delta
                                    );
                                    last_seek_detected = Some(Instant::now());
                                    detected_seek = Some((session_file_id, position));
                                }
                            }
                        }
                        last_position = position;
                    }
                } else {
                    // Reset stall counter when paused or dead to avoid false positives
                    if stall_counter > 0 {
                        info!(
                            "[stall] monitor: file={} reset counter (alive={} paused={} counter was {})",
                            session_file_id, session_alive, session.status.paused, stall_counter
                        );
                        stall_counter = 0;
                    }
                }
            }

            if !session_alive {
                teardown_session(&mut *guard, &state, &handle);
                should_break = true;
            }

            if let Some((file_id, position)) = detected_seek {
                if std::env::var("DEBUG").is_ok() {
                    info!("[seek] monitor detected native seek: file={} pos={}s", file_id, position);
                }
                let state = state.clone();
                handle.spawn(async move {
                    state.player_runtime.record_recent_seek_target(file_id, position).await;
                });
            }

            if should_break {
                break;
            }

            thread::sleep(Duration::from_millis(250));
        }
    });
}


pub async fn bridge_open_native_player(
    state: &PlayerContext,
    request: BridgeOpenPlayerRequest,
) -> Result<(), String> {
    debug_log!("mpv", "bridge_open_native_player start: file={} title={}", request.file_id, request.title);
    if !crate::infrastructure::native_player_runtime_ready() {
        let e = native_player_runtime_error();
        debug_log!("mpv", "bridge_open_native_player fail: {}", e);
        return Err(e);
    }

    let file_id = request.file_id;
    let title = request.title;
    let start_position_sec = request.start_position_sec;
    let session_type = request.session_type.unwrap_or(MpvSessionType::Video);
    let is_audio = session_type == MpvSessionType::Audio;

    // Phase 1: Ensure playback ready FIRST while old session is still in the slot,
    // so UI/monitor don't see an empty slot during slow async operations.
    let file = match crate::ensure_video_playback_ready(state, file_id).await {
        Ok(f) => f,
        Err(e) => {
            debug_log!("mpv", "bridge_open_native_player fail: ensure_video_playback_ready: {}", e);
            return Err(e);
        }
    };
    let original_parts = state.file_repo.get_original_parts_for_file(file_id)
        .await
        .unwrap_or_default();
    let has_chunk_parts = !original_parts.is_empty();

    if has_chunk_parts {
        warm_playback_runtime_state(state, file_id, original_parts).await;
    }

    let media_source = if has_chunk_parts {
        info!(
            "Bridge-owned mpv session ({:?}) will use cloud raw stream directly for '{}'.",
            session_type, file.filename
        );
        let ip = crate::infrastructure::pick_working_ip();
        let url = format!("http://{}:{}/raw/{}", ip, state.bridge_port.load(std::sync::atomic::Ordering::Relaxed), file_id);
        debug_log!("mpv", "bridge_open_native_player url: {}", url);
        url
    } else {
        let e = "File khong co original chunk de phat native player.".to_string();
        debug_log!("mpv", "bridge_open_native_player fail: {}", e);
        return Err(e);
    };

    let start_pos = normalize_start_position_sec(start_position_sec);
    let session_id = NEXT_NATIVE_MPV_SESSION_ID.fetch_add(1, Ordering::Relaxed);

    // Phase 2: Take old session out of the slot, release lock, then shutdown.
    // Lock is NOT held during shutdown_native_session so the monitor can still
    // check status (and will see the old session id -> break when we swap below).
    let old_session = {
        let mut guard = mpv_session_store_by_type(session_type)
            .lock()
            .map_err(|_| {
                let e = "Failed to lock native mpv session".to_string();
                debug_log!("mpv", "bridge_open_native_player fail: {}", e);
                e
            })?;
        if let Some(ref session) = guard.as_ref() {
            if session.file_id == file_id && session.status.alive {
                debug_log!(
                    "teardown",
                    "closing existing session for file {} before reopen",
                    file_id
                );
            }
        }
        // take_session_for_teardown refreshes status then sets slot = None.
        // The None window is very short (Phase 3 only, ~1ms).
        take_session_for_teardown(&mut *guard, state)
    }; // guard dropped -- lock released

    if let Some(mut session) = old_session {
        debug_log!("teardown", "shutting down old session file={} id={}", session.file_id, session.id);
        shutdown_native_session(&mut session, state);
        drop(session);
        debug_log!("teardown", "old session teardown complete for file {}", file_id);
    }

    // Phase 3: Init mpv on the current async thread (NOT spawn_blocking).
    // On Windows, creating the mpv rendering context on a pooled blocking thread
    // causes the window to break when the thread is recycled.
    let mut init_error = None::<String>;
    let mpv = libmpv2::Mpv::with_initializer(|init| {
        // gpu-next: required for RTX Video Smoothing + NVIDIA VSR
        let _ = init.set_option("vo", "gpu-next");

        let has_adapter = state.cfg.d3d11_adapter != "Auto";
        let hwdec_val = if has_adapter { "d3d11va" } else { "auto" };
        let _ = init.set_property("hwdec", hwdec_val);

        if has_adapter {
            let _ = init.set_option("d3d11-adapter", state.cfg.d3d11_adapter.as_str());
        }

        if is_audio {
            let _ = init.set_option("video", "no");
            let _ = init.set_option("force-window", "no");
            let _ = init.set_option("audio-display", "no");
        } else {
            if let Err(err) = configure_custom_player_ui(&init) {
                init_error = Some(err);
            }
        }

        if init_error.is_none() {
            if let Err(err) = configure_native_player_buffering(&init, state, has_chunk_parts) {
                init_error = Some(err);
            }
        }

        let _ = init.set_option("input-default-bindings", "yes");
        let _ = init.set_option("input-vo-keyboard", "yes");

        if !is_audio {
            let _ = init.set_option("hidpi-window-scale", "no");
            let _ = init.set_option("window-scale", "1");
            let _ = init.set_option("window-dragging", "no");
        }

        let _ = init.set_option("osd-bar", "no");

        if start_pos > 0.0 {
            let start_opt = format!("{start_pos}");
            let _ = init.set_option("start", start_opt.as_str());
        }

        let _ = init.set_option("ytdl", "no");

        Ok::<(), libmpv2::Error>(())
    })
    .map_err(|err| {
        let e = format_mpv_error("Failed to init mpv", err);
        debug_log!("mpv", "bridge_open_native_player fail: {}", e);
        e
    })?;

    if let Some(err) = init_error {
        debug_log!("mpv", "bridge_open_native_player fail: init_error={}", err);
        return Err(err);
    }

    // Phase 4: Post-init properties
    let _ = mpv.set_property("force-media-title", title.as_str());

    if !is_audio {
        let _ = mpv.set_property("ontop", true);
        let _ = mpv.observe_property("user-data/picker", libmpv2::Format::String, 6969);
        let _ = mpv.observe_property("aid", libmpv2::Format::Int64, 6970);
        let _ = mpv.observe_property("duration", libmpv2::Format::Double, 6971);
    }

    // Phase 5: loadfile -- libmpv queues this asynchronously (non-blocking).
    debug_log!("loadfile", "sending loadfile command for file {}", file_id);
    mpv.command("loadfile", &[media_source.as_str()])
        .map_err(|err| {
            let e = format_mpv_error("Failed to load media in mpv", err);
            debug_log!("mpv", "bridge_open_native_player fail: {}", e);
            e
        })?;

    // Phase 5b: Collect external audio tracks (deferred until file is loaded)
    let pending_audio_tracks = collect_pending_audio_tracks(state, file_id).await;

    // Phase 6: Build session and immediately fill the slot.
    let window_label = if is_audio {
        "audio-headless"
    } else {
        "native-mpv"
    };

    let mut new_session = NativeMpvSession {
        id: session_id,
        file_id,
        window_label: window_label.to_string(),
        title: title.clone(),
        status: MpvStatus {
            alive: true,
            paused: false,
            position: start_pos,
            duration: file.duration_sec.unwrap_or(0.0),
            volume: 100.0,
            speed: 1.0,
            fullscreen: false,
            title: title.clone(),
        },
        mpv,
        pending_default_audio: None,
        pending_audio_tracks,
        audio_tracks_loaded: false,
    };
    refresh_session_status(&mut new_session, state);
    if new_session.status.duration <= 0.0 {
        new_session.status.duration = file.duration_sec.unwrap_or(0.0);
    }

    {
        let mut guard = mpv_session_store_by_type(session_type)
            .lock()
            .map_err(|_| {
                let e = "Failed to lock native mpv session".to_string();
                debug_log!("mpv", "bridge_open_native_player fail: {}", e);
                e
            })?;
        *guard = Some(new_session);
    } // guard dropped

    // Phase 7: ontop-off delay, monitor, logging
    if !is_audio {
        let mpv_session_store = mpv_session_store_by_type(MpvSessionType::Video);
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            if let Ok(mut guard) = mpv_session_store.lock() {
                if let Some(session) = guard.as_mut() {
                    let _ = session.mpv.set_property("ontop", false);
                }
            }
        });
    }

    let handle = tokio::runtime::Handle::current();

    emit_playback_activity(state, true, window_label);

    // Delay monitor spawn by 500ms to give mpv time to initialize after loadfile
    // This prevents the monitor from polling properties before mpv is ready,
    // which would cause transient errors and position=0 stuck in the first tick
    let state_clone = state.clone();
    let handle_clone = handle.clone();
    handle.spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        spawn_session_monitor(state_clone, session_id, session_type, handle_clone);
    });

    info!(
        "Opened bridge-owned mpv session ({:?}) for file {} using source {}",
        session_type, file_id, media_source
    );
    debug_log!("mpv", "bridge_open_native_player done: file={}", file_id);

    Ok(())
}

async fn collect_pending_audio_tracks(
    state: &PlayerContext,
    file_id: i64,
) -> Vec<(String, String)> {
    let Ok(Some(video_meta)) = state.file_repo.get_video_file(file_id).await else {
        return Vec::new();
    };
    let ip = crate::infrastructure::pick_working_ip();
    let bridge_port = state.bridge_port.load(std::sync::atomic::Ordering::Relaxed);
    let mut tracks = Vec::new();
    let mut loaded_ids = Vec::new();

    if let Some(ref audio_json) = video_meta.audio {
        if let Ok(audio_ids) = serde_json::from_str::<Vec<i64>>(audio_json) {
            for &audio_file_id in &audio_ids {
                let audio_url = format!("http://{}:{}/raw/{}", ip, bridge_port, audio_file_id);
                let flag = if Some(audio_file_id) == video_meta.default_audio { "select" } else { "auto" };
                tracks.push((audio_url, flag.to_string()));
                loaded_ids.push(audio_file_id);
            }
        }
    }

    if let Some(default_id) = video_meta.default_audio {
        if !loaded_ids.contains(&default_id) {
            let audio_url = format!("http://{}:{}/raw/{}", ip, bridge_port, default_id);
            tracks.push((audio_url, "select".to_string()));
        }
    }

    tracks
}

pub async fn bridge_current_mpv_status(
    state: &PlayerContext,
    session_type: MpvSessionType,
) -> Result<MpvStatus, String> {
    current_mpv_status(state, session_type).await
}

pub async fn mpv_get_status(
    state: &PlayerContext,
    session_type: MpvSessionType,
) -> Result<MpvStatus, String> {
    bridge_current_mpv_status(state, session_type).await
}

pub async fn bridge_mpv_play_pause(
    state: &PlayerContext,
    session_type: MpvSessionType,
) -> Result<MpvStatus, String> {
    mutate_mpv_session(state, session_type, |session| {
        let next_paused = !session.status.paused;
        session
            .mpv
            .set_property("pause", next_paused)
            .map_err(|err| format_mpv_error("Failed to toggle mpv pause", err))
    })
    .await
}

pub async fn mpv_play_pause(
    state: &PlayerContext,
    session_type: MpvSessionType,
) -> Result<MpvStatus, String> {
    bridge_mpv_play_pause(state, session_type).await
}

pub async fn bridge_mpv_seek(
    state: &PlayerContext,
    position: f64,
    session_type: MpvSessionType,
) -> Result<MpvStatus, String> {
    let mut seek_file_id = None;
    let clamped = position.max(0.0);
    let status = mutate_mpv_session(state, session_type, |session| {
        seek_file_id = Some(session.file_id);
        let pos = format!("{clamped}");
        session
            .mpv
            .command("seek", &[pos.as_str(), "absolute"])
            .map_err(|err| format_mpv_error("Failed to seek mpv", err))
    })
    .await?;

    if std::env::var("DEBUG").is_ok() {
        info!("[mpv] seek: file={:?} pos={}s type={:?}", seek_file_id, clamped, session_type);
    }

    if let Some(file_id) = seek_file_id {
        state
            .player_runtime
            .record_recent_seek_target(file_id, clamped)
            .await;
    }

    Ok(status)
}

pub async fn mpv_seek(
    state: &PlayerContext,
    position: f64,
    session_type: MpvSessionType,
) -> Result<MpvStatus, String> {
    bridge_mpv_seek(state, position, session_type).await
}

pub async fn bridge_mpv_set_volume(
    state: &PlayerContext,
    volume: f64,
    session_type: MpvSessionType,
) -> Result<MpvStatus, String> {
    mutate_mpv_session(state, session_type, |session| {
        session
            .mpv
            .set_property("volume", volume.clamp(0.0, 100.0))
            .map_err(|err| format_mpv_error("Failed to set mpv volume", err))
    })
    .await
}

pub async fn mpv_set_volume(
    state: &PlayerContext,
    volume: f64,
    session_type: MpvSessionType,
) -> Result<MpvStatus, String> {
    bridge_mpv_set_volume(state, volume, session_type).await
}

pub async fn bridge_mpv_set_speed(
    state: &PlayerContext,
    speed: f64,
    session_type: MpvSessionType,
) -> Result<MpvStatus, String> {
    mutate_mpv_session(state, session_type, |session| {
        session
            .mpv
            .set_property("speed", speed.clamp(0.25, 4.0))
            .map_err(|err| format_mpv_error("Failed to set mpv speed", err))
    })
    .await
}

pub async fn mpv_set_speed(
    state: &PlayerContext,
    speed: f64,
    session_type: MpvSessionType,
) -> Result<MpvStatus, String> {
    bridge_mpv_set_speed(state, speed, session_type).await
}

pub async fn bridge_mpv_toggle_fullscreen(
    state: &PlayerContext,
    session_type: MpvSessionType,
) -> Result<MpvStatus, String> {
    mutate_mpv_session(state, session_type, |session| {
        let next = !session.status.fullscreen;
        session
            .mpv
            .set_property("fullscreen", next)
            .map_err(|err| format_mpv_error("Failed to toggle mpv fullscreen", err))
    })
    .await
}

pub async fn mpv_toggle_fullscreen(
    state: &PlayerContext,
    session_type: MpvSessionType,
) -> Result<MpvStatus, String> {
    bridge_mpv_toggle_fullscreen(state, session_type).await
}

pub async fn bridge_mpv_shutdown(
    state: &PlayerContext,
    session_type: MpvSessionType,
) -> Result<MpvStatus, String> {
    let session = {
        let mut guard = mpv_session_store_by_type(session_type)
            .lock()
            .map_err(|_| format!("Failed to lock native mpv {:?} session", session_type))?;

        take_session_for_teardown(&mut *guard, state)
    };

    if let Some(session) = session {
        teardown_taken_session_async(session, state, session_type).await;
    }

    Ok(MpvStatus::default())
}

pub async fn mpv_shutdown(
    state: &PlayerContext,
    session_type: MpvSessionType,
) -> Result<MpvStatus, String> {
    bridge_mpv_shutdown(state, session_type).await
}

pub async fn player_update_playback_progress(
    
    state: &PlayerContext,
    file_id: i64,
    position: f64,
) -> Result<(), String> {
    state.file_repo.save_playback_history(file_id, position, None, false)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn player_clear_playback_history(
    
    state: &PlayerContext,
    file_id: i64,
) -> Result<(), String> {
    state.file_repo.clear_playback_history(file_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn open_in_native_player(
    
    state: &PlayerContext,
    file_id: i64,
    title: String,
    start_position_sec: Option<f64>,
    session_type: Option<MpvSessionType>,
) -> Result<(), String> {
    bridge_open_native_player(
        state,
        BridgeOpenPlayerRequest {
            file_id,
            title,
            start_position_sec,
            session_type,
        },
    )
    .await
}

pub async fn mpv_add_audio_track(
    state: &PlayerContext,
    video_file_id: i64,
    audio_file_id: i64,
) -> Result<(), String> {
    let session_type = MpvSessionType::Video;
    let ip = crate::infrastructure::pick_working_ip();
    let url = format!(
        "http://{}:{}/raw/{}",
        ip,
        state.bridge_port.load(std::sync::atomic::Ordering::Relaxed),
        audio_file_id
    );

    mutate_mpv_session(state, session_type, |session| {
        if session.file_id != video_file_id {
            return Err("Session is playing a different file".to_string());
        }
        session
            .mpv
            .command("audio-add", &[url.as_str(), "auto"])
            .map_err(|e| format!("Failed to add audio track: {}", e))
    })
    .await?;

    Ok(())
}
