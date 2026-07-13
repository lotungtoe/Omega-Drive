use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use omega_drive_gateway::provider::ui_events::UiEventEmitter;
use tokio::sync::mpsc;

use crate::progress::{emit_progress_with_platforms, platform_progress};

const UPLOAD_EVENT_NAME: &str = "upload-progress";

#[derive(Clone)]
pub(crate) struct UploadSessionTracker {
    emitter: Arc<dyn UiEventEmitter>,
    session_id: String,
    file_name: String,
    file_id: Arc<Mutex<Option<i64>>>,
    total_parts: Arc<AtomicUsize>,
    total_bytes: Arc<AtomicU64>,
    parts_done: Arc<AtomicUsize>,
    platform_totals: Arc<Mutex<HashMap<String, u64>>>,
    platform_done: Arc<Mutex<HashMap<String, u64>>>,
}

impl UploadSessionTracker {
    #[allow(dead_code)]
    pub(crate) fn session_id(&self) -> &str {
        &self.session_id
    }

    fn lock_platform_totals(&self) -> MutexGuard<'_, HashMap<String, u64>> {
        match self.platform_totals.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn lock_platform_done(&self) -> MutexGuard<'_, HashMap<String, u64>> {
        match self.platform_done.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    pub(crate) fn new(emitter: Arc<dyn UiEventEmitter>, session_id: String, file_name: String) -> Self {
        Self {
            emitter, session_id, file_name,
            file_id: Arc::new(Mutex::new(None)),
            total_parts: Arc::new(AtomicUsize::new(0)),
            total_bytes: Arc::new(AtomicU64::new(0)),
            parts_done: Arc::new(AtomicUsize::new(0)),
            platform_totals: Arc::new(Mutex::new(HashMap::new())),
            platform_done: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub(crate) fn set_file_id(&self, file_id: i64) {
        match self.file_id.lock() {
            Ok(mut guard) => *guard = Some(file_id),
            Err(poisoned) => *poisoned.into_inner() = Some(file_id),
        }
    }

    pub(crate) fn configure(&self, total_parts: usize, total_bytes: u64, per_platform_totals: HashMap<String, u64>) {
        self.total_parts.store(total_parts, Ordering::Relaxed);
        self.total_bytes.store(total_bytes, Ordering::Relaxed);
        let mut totals = self.lock_platform_totals();
        *totals = per_platform_totals;
        let mut done = self.lock_platform_done();
        done.clear();
    }

    pub(crate) fn emit_preparing(&self) {
        let (totals, done) = (self.lock_platform_totals().clone(), self.lock_platform_done().clone());
        self.emit("preparing", 0, 1, "Preparing upload...", &totals, &done);
    }

    pub(crate) fn emit_finalizing_integrity(&self) {
        let total_parts = self.total_parts.load(Ordering::Relaxed);
        let (totals, done) = (self.lock_platform_totals().clone(), self.lock_platform_done().clone());
        self.emit("finalizing_integrity", total_parts, total_parts, "Finalizing file integrity...", &totals, &done);
    }

    pub(crate) fn record_existing_part(&self, platform: &str, size: u64) {
        let mut done = self.lock_platform_done();
        let current = done.entry(platform.to_string()).or_insert(0);
        *current += size;
        self.parts_done.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn spawn_telegram_progress_listener(&self) -> mpsc::UnboundedSender<usize> {
        let (tx, mut rx) = mpsc::unbounded_channel::<usize>();
        let session = self.clone();

        tokio::spawn(async move {
            let mut last_emitted = 0u64;
            let threshold = 5 * 1024 * 1024;
            while let Some(bytes) = rx.recv().await {
                let telegram_total = { let totals = session.lock_platform_totals(); *totals.get("telegram").unwrap_or(&0) };
                let current = {
                    let mut done = session.lock_platform_done();
                    let val = done.entry("telegram".to_string()).or_insert(0);
                    let updated = val.saturating_add(bytes as u64);
                    *val = if telegram_total > 0 { updated.min(telegram_total) } else { updated };
                    *val
                };
                if current.saturating_sub(last_emitted) >= threshold || current == telegram_total {
                    last_emitted = current;
                    session.emit_with_current("uploading", session.parts_done.load(Ordering::Relaxed), "Uploading to Telegram...");
                }
            }
        });
        tx
    }

    pub(crate) fn complete_part(&self, platform: &str, delta: u64) {
        if delta > 0 {
            let mut done = self.lock_platform_done();
            let val = done.entry(platform.to_string()).or_insert(0);
            *val += delta;
        }
        self.finish_part();
    }

    pub(crate) fn complete_part_without_bytes(&self) { self.finish_part(); }

    fn finish_part(&self) {
        let done_partss = self.parts_done.fetch_add(1, Ordering::Relaxed) + 1;
        let total_parts = self.total_parts.load(Ordering::Relaxed);
        let detail = format!("Uploading parts ({}/{})", done_partss, total_parts);
        self.emit_with_current("uploading", done_partss, &detail);
    }

    pub(crate) fn emit_processing(&self) {
        let total_parts = self.total_parts.load(Ordering::Relaxed);
        let (totals, done) = (self.lock_platform_totals().clone(), self.lock_platform_done().clone());
        self.emit("processing", total_parts, total_parts, "Processing derivatives...", &totals, &done);
    }

    pub(crate) fn emit_done(&self) {
        let total_parts = self.total_parts.load(Ordering::Relaxed);
        let (totals, done) = (self.lock_platform_totals().clone(), self.lock_platform_done().clone());
        self.emit("done", total_parts, total_parts, "Upload complete", &totals, &done);
    }

    pub(crate) fn emit_failed(&self, detail: &str) {
        let done_partss = self.parts_done.load(Ordering::Relaxed);
        let (totals, done) = (self.lock_platform_totals().clone(), self.lock_platform_done().clone());
        self.emit("failed", done_partss, self.total_parts.load(Ordering::Relaxed), detail, &totals, &done);
    }

    fn emit_with_current(&self, phase: &str, done_parts: usize, detail: &str) {
        let (totals, done) = (self.lock_platform_totals().clone(), self.lock_platform_done().clone());
        self.emit(phase, done_parts, self.total_parts.load(Ordering::Relaxed), detail, &totals, &done);
    }

    fn emit(&self, phase: &str, done_parts: usize, total_parts: usize, detail: &str, totals: &HashMap<String, u64>, done: &HashMap<String, u64>) {
        let mut platforms = Vec::new();
        let providers = ["discord", "telegram"];
        for p_name in providers {
            if let Some(&total) = totals.get(p_name) {
                if total > 0 {
                    let d = *done.get(p_name).unwrap_or(&0);
                    let display_name = match p_name { "discord" => "Discord", "telegram" => "Telegram", _ => p_name };
                    if let Some(platform) = platform_progress(display_name, d, total) {
                        platforms.push(platform);
                    }
                }
            }
        }
        let file_id = match self.file_id.lock() {
            Ok(guard) => *guard,
            Err(poisoned) => *poisoned.into_inner(),
        };
        emit_progress_with_platforms(self.emitter.as_ref(), UPLOAD_EVENT_NAME, &self.session_id, &self.file_name, phase, done_parts, total_parts, detail, platforms, file_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omega_drive_gateway::provider::ui_events::UiEventEmitter;
    use serde_json::Value;
    use tokio::{task::yield_now, time::{timeout, Duration}};

    struct NoopUiEventEmitter;
    impl UiEventEmitter for NoopUiEventEmitter {
        fn emit_value(&self, _event_name: &str, _payload: Value) {}
    }

    async fn wait_for_platform_done(session: &UploadSessionTracker, platform: &str, expected: u64) {
        timeout(Duration::from_millis(200), async {
            loop {
                let current = { let done = session.platform_done.lock().unwrap(); *done.get(platform).unwrap_or(&0) };
                if current == expected { break; }
                yield_now().await;
            }
        }).await.expect("timed out waiting for upload progress");
    }

    #[test]
    fn complete_part_adds_bytes_for_non_streaming_platforms() {
        let session = UploadSessionTracker::new(Arc::new(NoopUiEventEmitter), "test-session".to_string(), "test_file.mp4".to_string());
        let mut totals = HashMap::new();
        totals.insert("discord".to_string(), 42);
        session.configure(1, 42, totals);
        session.complete_part("discord", 42);
        let done = session.platform_done.lock().unwrap();
        assert_eq!(done.get("discord"), Some(&42));
        assert_eq!(session.parts_done.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn telegram_streamed_progress_is_clamped_and_not_double_counted() {
        let session = UploadSessionTracker::new(Arc::new(NoopUiEventEmitter), "test-session".to_string(), "test_file.mp4".to_string());
        let mut totals = HashMap::new();
        totals.insert("telegram".to_string(), 100);
        session.configure(1, 100, totals);
        let tx = session.spawn_telegram_progress_listener();
        tx.send(80).unwrap();
        tx.send(80).unwrap();
        drop(tx);
        wait_for_platform_done(&session, "telegram", 100).await;
        session.complete_part_without_bytes();
        let done = session.platform_done.lock().unwrap();
        assert_eq!(done.get("telegram"), Some(&100));
        assert_eq!(session.parts_done.load(Ordering::Relaxed), 1);
    }
}
