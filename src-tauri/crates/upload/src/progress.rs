use std::sync::Arc;

use omega_drive_gateway::provider::app_context::AppContext;
use omega_drive_gateway::provider::ui_events::UiEventEmitter;
use omega_drive_gateway::provider::ui_events::emit_serialized;
use omega_drive_gateway::core::types::PlatformProgress;
use serde::Serialize;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProgressPayload {
    pub session_id: String,
    pub file_name: String,
    pub phase: String,
    pub done_parts: usize,
    pub total_parts: usize,
    pub detail: String,
    pub overall_progress: f64,
    pub platforms: Vec<PlatformProgress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<i64>,
}

#[allow(clippy::too_many_arguments)]
pub fn emit_progress(
    app_ctx: Arc<dyn AppContext>,
    event_name: &str,
    session_id: &str,
    file_name: &str,
    phase: &str,
    done_parts: usize,
    total_parts: usize,
    detail: &str,
    discord_done: u64,
    discord_total: u64,
    telegram_done: u64,
    telegram_total: u64,
    file_id: Option<i64>,
) {
    let emitter = AppContextUiEventEmitter(app_ctx);
    emit_progress_to(&emitter, event_name, session_id, file_name, phase, done_parts, total_parts, detail, discord_done, discord_total, telegram_done, telegram_total, file_id);
}

pub(crate) fn app_event_emitter(state: &super::context::UploadContext) -> Arc<dyn UiEventEmitter> {
    Arc::new(AppContextUiEventEmitter(state.app_ctx.clone()))
}

pub(crate) fn platform_progress(name: &str, done: u64, total: u64) -> Option<PlatformProgress> {
    (total > 0).then(|| PlatformProgress { name: name.to_string(), done, total })
}

pub(crate) fn emit_progress_with_platforms(
    emitter: &dyn UiEventEmitter,
    event_name: &str,
    session_id: &str,
    file_name: &str,
    phase: &str,
    done_parts: usize,
    total_parts: usize,
    detail: &str,
    platforms: Vec<PlatformProgress>,
    file_id: Option<i64>,
) {
    let (done_bytes, total_bytes) = platforms.iter().fold((0, 0), |(done, total), platform| {
        (done + platform.done, total + platform.total)
    });

    let overall_progress = if total_bytes > 0 {
        (done_bytes as f64 / total_bytes as f64) * 100.0
    } else if total_parts > 0 {
        (done_parts as f64 / total_parts as f64) * 100.0
    } else {
        0.0
    };

    let payload = ProgressPayload {
        session_id: session_id.to_string(),
        file_name: file_name.to_string(),
        phase: phase.to_string(),
        done_parts,
        total_parts,
        detail: detail.to_string(),
        overall_progress,
        platforms,
        file_id,
    };

    emit_serialized(emitter, event_name, &payload);
}

pub(crate) fn emit_progress_to(
    emitter: &dyn UiEventEmitter,
    event_name: &str,
    session_id: &str,
    file_name: &str,
    phase: &str,
    done_parts: usize,
    total_parts: usize,
    detail: &str,
    discord_done: u64,
    discord_total: u64,
    telegram_done: u64,
    telegram_total: u64,
    file_id: Option<i64>,
) {
    let mut platforms = Vec::new();
    if let Some(platform) = platform_progress("Discord", discord_done, discord_total) {
        platforms.push(platform);
    }
    if let Some(platform) = platform_progress("Telegram", telegram_done, telegram_total) {
        platforms.push(platform);
    }
    emit_progress_with_platforms(emitter, event_name, session_id, file_name, phase, done_parts, total_parts, detail, platforms, file_id);
}

struct AppContextUiEventEmitter(Arc<dyn AppContext>);

impl UiEventEmitter for AppContextUiEventEmitter {
    fn emit_value(&self, event_name: &str, payload: serde_json::Value) {
        self.0.emit_event(event_name, payload);
    }
}
