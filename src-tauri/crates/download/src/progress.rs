use omega_drive_gateway::provider::app_context::AppContext;
use std::sync::Arc;

// ponytail: inline progress emission — no shared progress module needed
pub(crate) fn emit_progress(
    app_ctx: Arc<dyn AppContext>,
    event_name: &str,
    session_id: &str,
    filename: &str,
    phase: &str,
    done_parts: usize,
    total_parts: usize,
    detail: &str,
    bytes_done: u64,
    bytes_total: u64,
    done: u64,
    total: u64,
    file_id: Option<i64>,
) {
    app_ctx.emit_event(
        event_name,
        serde_json::json!({
            "sessionId": session_id,
            "fileName": filename,
            "phase": phase,
            "doneParts": done_parts,
            "totalParts": total_parts,
            "detail": detail,
            "bytesDone": bytes_done,
            "bytesTotal": bytes_total,
            "done": done,
            "total": total,
            "fileId": file_id,
        }),
    );
}
