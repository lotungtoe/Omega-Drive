use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Mutex, Notify};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use omega_drive_gateway::core::error::AppResult;
use omega_drive_gateway::core::error::wrap_error;
use omega_drive_gateway::core::error_codes as codes;
use omega_drive_gateway::provider::storage::PartMetadata;
use omega_drive_gateway::core::data::DownloadJob;

use crate::{
    build_temp_path, run_download_job, DownloadCompletion, DownloadContext, DownloadJobError,
};

// â”€â”€ PowerGuard (inlined â€” original: src/infrastructure/power_guard.rs) â”€â”€

struct PowerGuard {
    count: AtomicUsize,
}

impl PowerGuard {
    fn new() -> Self {
        Self {
            count: AtomicUsize::new(0),
        }
    }

    fn acquire(&self) {
        let prev = self.count.fetch_add(1, Ordering::SeqCst);
        if prev == 0 {
            set_sleep_block(true);
        }
    }

    fn release(&self) {
        let prev = self.count.fetch_sub(1, Ordering::SeqCst);
        if prev == 0 {
            self.count.store(0, Ordering::SeqCst);
            return;
        }
        if prev == 1 {
            set_sleep_block(false);
        }
    }
}

#[cfg(target_os = "windows")]
fn set_sleep_block(block: bool) {
    use windows_sys::Win32::System::Power::{
        SetThreadExecutionState, ES_CONTINUOUS, ES_SYSTEM_REQUIRED,
    };
    // SAFETY: SetThreadExecutionState is a process-local Win32 API with no raw
    // pointers here. We only toggle the documented execution-state flags.
    unsafe {
        if block {
            let _ = SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED);
        } else {
            let _ = SetThreadExecutionState(ES_CONTINUOUS);
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn set_sleep_block(_block: bool) {}

// â”€â”€ DownloadManager â”€â”€

pub struct DownloadManager {
    inflight_file_ids: Mutex<HashSet<i64>>,
    running_tokens: Mutex<HashMap<i64, CancellationToken>>,
    notify: Notify,
    power_guard: PowerGuard,
}

impl DownloadManager {
    pub fn new() -> Self {
        Self {
            inflight_file_ids: Mutex::new(HashSet::new()),
            running_tokens: Mutex::new(HashMap::new()),
            notify: Notify::new(),
            power_guard: PowerGuard::new(),
        }
    }

    pub fn start(self: Arc<Self>, state: DownloadContext) {
        let mgr = Arc::clone(&self);
        let st = state.clone();
        tokio::spawn(async move {
            mgr.init_startup(&st).await;
            mgr.run_loop(st).await;
        });

        let purge_mgr = Arc::clone(&self);
        let purge_state = state.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(24 * 3600)).await;
                if let Err(e) = purge_mgr.purge_old_jobs(&purge_state).await {
                    warn!("Download purge failed: {}", e);
                }
            }
        });
    }

    async fn init_startup(&self, state: &DownloadContext) {
        let (purge_days, auto_resume) = {
            let cfg = state.cfg.read().expect("cfg RwLock");
            (cfg.purge_days as i64, cfg.auto_resume_on_startup)
        };
        if let Err(e) = state
            .download_job_repo
            .pause_all_active_jobs("shutdown")
            .await
        {
            warn!("Failed to pause active download jobs: {}", e);
        }
        if auto_resume {
            if let Err(e) = state.download_job_repo.resume_shutdown_jobs().await {
                warn!("Failed to resume shutdown jobs: {}", e);
            }
        }
        let _ = state
            .download_job_repo
            .purge_old_jobs(purge_days, &["failed", "cancelled"])
            .await;
        self.notify.notify_one();
    }

    async fn purge_old_jobs(&self, state: &DownloadContext) -> anyhow::Result<()> {
        let purge_days = state.cfg.read().expect("cfg RwLock").purge_days as i64;
        state
            .download_job_repo
            .purge_old_jobs(purge_days, &["failed", "cancelled"])
            .await?;
        Ok(())
    }

    async fn run_loop(&self, state: DownloadContext) {
        loop {
            let job_opt = state
                .download_job_repo
                .get_next_queued()
                .await
                .ok()
                .flatten();

            let Some(job) = job_opt else {
                self.notify.notified().await;
                continue;
            };

            if let Err(e) = state
                .download_job_repo
                .update_state(job.id, "downloading", None, None)
                .await
            {
                warn!("Failed to update job state: {}", e);
            }

            let token = CancellationToken::new();
            {
                let mut running = self.running_tokens.lock().await;
                running.insert(job.id, token.clone());
            }

            if state.cfg.read().expect("cfg RwLock").prevent_sleep_enabled {
                self.power_guard.acquire();
            }

            let result = run_download_job(state.clone(), job.clone(), token.clone()).await;

            if state.cfg.read().expect("cfg RwLock").prevent_sleep_enabled {
                self.power_guard.release();
            }

            {
                let mut running = self.running_tokens.lock().await;
                running.remove(&job.id);
            }

            match result {
                Ok(DownloadCompletion {
                    file_id,
                    filename,
                    target_path,
                }) => {
                    let _ = state.download_job_repo.delete_job(job.id).await;
                    self.emit_complete(&state, file_id, &filename, &target_path);
                }
                Err(DownloadJobError::DiskFull) => {
                    let _ = state
                        .download_job_repo
                        .update_state(job.id, "paused", Some("Disk full"), Some("disk_full"))
                        .await;
                    self.emit_failed(&state, job.file_id, "Disk full");
                }
                Err(DownloadJobError::Cancelled) => {}
                Err(DownloadJobError::Other(err)) => {
                    let _ = state
                        .download_job_repo
                        .update_state(job.id, "failed", Some(&err.to_string()), None)
                        .await;
                    self.emit_failed(&state, job.file_id, &err.to_string());
                }
            }
        }
    }

    pub async fn queue_download(
        &self,
        state: DownloadContext,
        file_id: i64,
        target_path: String,
    ) -> AppResult<DownloadJob> {
        let ctx = serde_json::json!({
            "feature": "download",
            "action": "queue_download",
            "file_id": file_id,
        });

        {
            let mut inflight = self.inflight_file_ids.lock().await;
            if inflight.contains(&file_id) {
                return Err(wrap_error(
                    "download",
                    codes::E_CONFLICT,
                    "Download task is being created.",
                    ctx,
                    anyhow::anyhow!("duplicate queue"),
                ));
            }
            inflight.insert(file_id);
        }

        let result = self
            .queue_download_inner(state.clone(), file_id, &target_path)
            .await;

        {
            let mut inflight = self.inflight_file_ids.lock().await;
            inflight.remove(&file_id);
        }

        if result.is_ok() {
            self.notify.notify_one();
        }
        result
    }

    async fn queue_download_inner(
        &self,
        state: DownloadContext,
        file_id: i64,
        target_path: &str,
    ) -> AppResult<DownloadJob> {
        let ctx = serde_json::json!({
            "feature": "download",
            "action": "queue_download",
            "file_id": file_id,
        });

        let (_file_info, parts) = {
            let file = state
                .file_repo
                .get_file_by_id(file_id)
                .await
                .map_err(|e| {
                    wrap_error("download", codes::E_DB, "DB error when fetching file.", ctx.clone(), e)
                })?
                .ok_or_else(|| {
                    wrap_error(
                        "download",
                        codes::E_NOT_FOUND,
                        "File not found in DB.",
                        ctx.clone(),
                        anyhow::anyhow!("file not found"),
                    )
                })?;
            let parts = state
                .file_repo
                .get_original_parts_for_file(file_id)
                .await
                .map_err(|e| {
                    wrap_error(
                        "download",
                        codes::E_DB,
                        "DB error when fetching parts.",
                        ctx.clone(),
                        e,
                    )
                })?;
            (file, parts)
        };

        let unique_parts = build_unique_parts(parts);
        let total_parts_unique = unique_parts.len() as i64;

        let desired_path = PathBuf::from(target_path);
        let final_path = resolve_unique_path(&desired_path);

        if state
            .download_job_repo
            .exists_active_job_for_file(file_id)
            .await
            .map_err(|e| {
                wrap_error(
                    "download",
                    codes::E_DB,
                    "DB error when checking job.",
                    ctx.clone(),
                    e,
                )
            })? {
            return Err(wrap_error(
                "download",
                codes::E_CONFLICT,
                "File is already downloading.",
                ctx.clone(),
                anyhow::anyhow!("active job exists"),
            ));
        }

        let job_id = state
            .download_job_repo
            .create_job(
                file_id,
                final_path.to_string_lossy().as_ref(),
                total_parts_unique,
            )
            .await
            .map_err(|e| {
                wrap_error("download", codes::E_DB, "DB error when creating job.", ctx.clone(), e)
            })?;

        let job = state
            .download_job_repo
            .get_job(job_id)
            .await
            .map_err(|e| {
                wrap_error("download", codes::E_DB, "DB error when fetching job.", ctx.clone(), e)
            })?
            .ok_or_else(|| {
                wrap_error(
                    "download",
                    codes::E_NOT_FOUND,
                    "Newly created job not found.",
                    ctx.clone(),
                    anyhow::anyhow!("job not found"),
                )
            })?;

        info!(
            "Queued download job {} for file {} -> {}",
            job_id,
            file_id,
            final_path.display()
        );
        self.emit_queued(&state, &job);
        Ok(job)
    }

    pub async fn pause_download(&self, state: DownloadContext, job_id: i64) -> AppResult<()> {
        let ctx = serde_json::json!({
            "feature": "download",
            "action": "pause_download",
            "job_id": job_id,
        });

        state
            .download_job_repo
            .update_state(job_id, "paused", None, Some("user"))
            .await
            .map_err(|e| {
                wrap_error(
                    "download",
                    codes::E_DB,
                    "DB error when pausing job.",
                    ctx.clone(),
                    e,
                )
            })?;

        self.cancel_running(job_id).await;
        Ok(())
    }

    pub async fn resume_download(&self, state: DownloadContext, job_id: i64) -> AppResult<()> {
        self.set_state_queued(&state, job_id, "resume_download")
            .await
    }

    pub async fn retry_download(&self, state: DownloadContext, job_id: i64) -> AppResult<()> {
        self.set_state_queued(&state, job_id, "retry_download")
            .await
    }

    async fn set_state_queued(
        &self,
        state: &DownloadContext,
        job_id: i64,
        action: &str,
    ) -> AppResult<()> {
        let ctx = serde_json::json!({
            "feature": "download",
            "action": action,
            "job_id": job_id,
        });
        state
            .download_job_repo
            .update_state(job_id, "queued", None, None)
            .await
            .map_err(|e| {
                wrap_error(
                    "download",
                    codes::E_DB,
                    &format!("DB error when {} job.", action.replace("_", " ")),
                    ctx,
                    e,
                )
            })?;
        self.notify.notify_one();
        Ok(())
    }

    pub async fn cancel_download(&self, state: DownloadContext, job_id: i64) -> AppResult<()> {
        let ctx = serde_json::json!({
            "feature": "download",
            "action": "cancel_download",
            "job_id": job_id,
        });

        let target_path = state
            .download_job_repo
            .get_job(job_id)
            .await
            .map_err(|e| {
                wrap_error(
                    "download",
                    codes::E_DB,
                    "DB error when fetching job.",
                    ctx.clone(),
                    e,
                )
            })?
            .map(|j| j.target_path);

        state
            .download_job_repo
            .update_state(job_id, "cancelled", Some("Cancelled by user"), None)
            .await
            .map_err(|e| {
                wrap_error(
                    "download",
                    codes::E_DB,
                    "DB error when cancelling job.",
                    ctx.clone(),
                    e,
                )
            })?;

        if let Some(path) = target_path {
            let temp_path = build_temp_path(Path::new(&path));
            if temp_path.exists() {
                let _ = std::fs::remove_file(&temp_path);
            }
        }

        self.cancel_running(job_id).await;
        Ok(())
    }

    pub async fn list_download_jobs(
        &self,
        state: DownloadContext,
    ) -> AppResult<Vec<DownloadJob>> {
        let ctx = serde_json::json!({
            "feature": "download",
            "action": "list_download_jobs",
        });
        let states = ["queued", "downloading", "paused", "failed"];
        let jobs = state
            .download_job_repo
            .list_jobs_by_state(&states)
            .await
            .map_err(|e| {
                wrap_error(
                    "download",
                    codes::E_DB,
                    "DB error when fetching job.",
                    ctx.clone(),
                    e,
                )
            })?;
        Ok(jobs)
    }

    async fn cancel_running(&self, job_id: i64) {
        let token_opt = {
            let running = self.running_tokens.lock().await;
            running.get(&job_id).cloned()
        };
        if let Some(token) = token_opt {
            token.cancel();
        }
    }

    fn emit_event(&self, state: &DownloadContext, event_name: &str, payload: serde_json::Value) {
        state.app_ctx.emit_event(event_name, payload);
    }

    fn emit_complete(
        &self,
        state: &DownloadContext,
        file_id: i64,
        filename: &str,
        target_path: &str,
    ) {
        self.emit_event(
            state,
            "download-complete",
            serde_json::json!({
                "fileId": file_id,
                "filename": filename,
                "path": target_path,
            }),
        );
        // ponytail: system notification handled by Tauri crate listening for "download-complete"
    }

    fn emit_queued(&self, state: &DownloadContext, job: &DownloadJob) {
        self.emit_event(
            state,
            "download-queued",
            serde_json::json!({
                "jobId": job.id,
                "fileId": job.file_id,
                "targetPath": job.target_path,
                "state": job.state,
            }),
        );
    }

    fn emit_failed(&self, state: &DownloadContext, file_id: i64, error_msg: &str) {
        self.emit_event(
            state,
            "download-failed",
            serde_json::json!({
                "fileId": file_id,
                "error": error_msg,
            }),
        );
    }
}

fn build_unique_parts(parts: Vec<PartMetadata>) -> Vec<PartMetadata> {
    let mut unique_parts_map = std::collections::BTreeMap::new();
    for p in parts {
        use std::collections::btree_map::Entry;
        match unique_parts_map.entry(p.part_index) {
            Entry::Vacant(e) => {
                e.insert(p);
            }
            Entry::Occupied(mut e) => {
                if p.platform == "telegram" && e.get().platform == "discord" {
                    e.insert(p);
                }
            }
        }
    }
    unique_parts_map.into_values().collect()
}

fn resolve_unique_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let dir = path.parent().unwrap_or_else(|| Path::new(""));
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    let ext = path.extension().map(|e| e.to_string_lossy().to_string());
    for i in 1..=9999 {
        let candidate = if let Some(ref ext) = ext {
            dir.join(format!("{stem} ({i}).{ext}"))
        } else {
            dir.join(format!("{stem} ({i})"))
        };
        if !candidate.exists() {
            return candidate;
        }
    }
    path.to_path_buf()
}
