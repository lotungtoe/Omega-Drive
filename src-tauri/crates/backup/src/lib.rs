use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use chrono::Utc;
pub use omega_drive_gateway::core::backup::{FilePayload, Op};
use omega_drive_gateway::provider::backup_service::BackupService as BackupServiceTrait;
use omega_drive_gateway::provider::discord_backup::DiscordBackupBackend;
use tracing::{error, info, warn};

const PENDING_FILE: &str = "backup_pending.jsonl";
const SEQ_FILE: &str = "backup_seq";

struct BackupInner {
    pending: Vec<Op>,
}

pub struct BackupService {
    inner: Mutex<BackupInner>,
    base_dir: PathBuf,
    next_seq: AtomicU64,
    pub backup_discord_thread_ids: Mutex<Vec<u64>>,
    pub backup_telegram_chat_id: Mutex<Option<i64>>,
}

impl BackupService {
    pub fn new(base_dir: PathBuf) -> Self {
        let seq = load_seq(&base_dir.join(SEQ_FILE)).unwrap_or(0);
        retry_staging_files(&base_dir);
        let pending = load_pending(&base_dir.join(PENDING_FILE));
        Self {
            inner: Mutex::new(BackupInner { pending }),
            base_dir,
            next_seq: AtomicU64::new(seq),
            backup_discord_thread_ids: Mutex::new(Vec::new()),
            backup_telegram_chat_id: Mutex::new(None),
        }
    }

    pub fn push_backup_thread(&self, thread_id: u64) -> Option<u64> {
        let mut ids = self.backup_discord_thread_ids.lock().expect("Mutex poisoned");
        ids.push(thread_id);
        if ids.len() > 5 {
            Some(ids.remove(0))
        } else {
            None
        }
    }

    pub fn latest_backup_thread_id(&self) -> Option<u64> {
        self.backup_discord_thread_ids.lock().expect("Mutex poisoned").last().copied()
    }

    pub fn set_backup_telegram_chat(&self, chat_id: i64) {
        *self.backup_telegram_chat_id.lock().expect("Mutex poisoned") = Some(chat_id);
    }
}

impl BackupServiceTrait for BackupService {
    fn next_seq(&self) -> u64 {
        self.next_seq.fetch_add(1, Ordering::Relaxed)
    }

    fn push_op(&self, op: Op) {
        let mut inner = self.inner.lock().expect("Mutex poisoned");
        persist_op_to_file(&self.base_dir.join(PENDING_FILE), &op);
        inner.pending.push(op);
    }

    fn flush_queues(&self) {
        let (ops, staging) = {
            let mut inner = self.inner.lock().expect("Mutex poisoned");
            if inner.pending.is_empty() {
                return;
            }
            let pending_path = self.base_dir.join(PENDING_FILE);
            let ts = chrono::Utc::now().timestamp_micros();
            let staging = self.base_dir.join(format!("ops_{}.pending.jsonl", ts));
            if let Err(e) = fs::rename(&pending_path, &staging) {
                error!("Backup: failed to rename pending file: {e}");
                return;
            }
            (std::mem::take(&mut inner.pending), staging)
        };

        let mut ops = ops;
        ops.sort_by_key(|op| op.seq());

        let mut content = String::new();
        for op in &ops {
            if let Ok(line) = serde_json::to_string(op) {
                content.push_str(&line);
                content.push('\n');
            }
        }

        let committed = staging.with_extension("jsonl");

        if let Err(e) = fs::write(&staging, &content) {
            error!("Backup: failed to write staging file: {e}");
            let mut inner = self.inner.lock().expect("Mutex poisoned");
            inner.pending.extend(ops);
            return;
        }

        info!("Backup: wrote {} ops to staging file {:?}", ops.len(), staging);

        if let Err(e) = fs::rename(&staging, &committed) {
            error!("Backup: failed to rename staging to committed: {e}");
            let mut inner = self.inner.lock().expect("Mutex poisoned");
            inner.pending.extend(ops);
            return;
        }

        persist_seq(&self.base_dir.join(SEQ_FILE), self.next_seq.load(Ordering::Relaxed));
    }

    fn has_pending(&self) -> bool {
        let inner = self.inner.lock().expect("Mutex poisoned");
        !inner.pending.is_empty()
    }

    fn try_capture_file(&self, _file_id: i64, _status: &str) {
        // ponytail: no-op â€” the app layer drives backup capture via the free function
        // which has access to a Connection. Proper capture needs DB access not available
        // in this crate's trait impl.
    }
}

fn persist_op_to_file(path: &PathBuf, op: &Op) {
    let line = match serde_json::to_string(op) {
        Ok(l) => l,
        Err(e) => {
            error!("Backup: failed to serialize op: {e}");
            return;
        }
    };
    match fs::OpenOptions::new().create(true).append(true).open(path) {
        Ok(mut f) => {
            let _ = writeln!(f, "{line}");
            let _ = f.flush();
        }
        Err(e) => error!("Backup: failed to write to pending file: {e}"),
    }
}

fn load_pending(path: &PathBuf) -> Vec<Op> {
    if !path.exists() {
        return Vec::new();
    }
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) => {
            error!("Backup: failed to open pending file for recovery: {e}");
            return Vec::new();
        }
    };
    let reader = BufReader::new(file);
    let mut ops = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                warn!("Backup: skipping unreadable line {}: {e}", i + 1);
                continue;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Op>(&line) {
            Ok(op) => ops.push(op),
            Err(e) => warn!("Backup: skipping malformed line {}: {e}", i + 1),
        }
    }
    if !ops.is_empty() {
        info!("Backup: recovered {} pending ops from {}", ops.len(), path.display());
    }
    ops
}

fn retry_staging_files(base_dir: &PathBuf) {
    let entries = match fs::read_dir(base_dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("pending") {
            continue;
        }
        let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        if !file_stem.starts_with("ops_") {
            continue;
        }
        let committed = path.with_extension("jsonl");
        info!("Backup: retrying staging file {:?}", path);
        if let Err(e) = fs::rename(&path, &committed) {
            error!("Backup: failed to retry staging file {path:?}: {e}");
        } else {
            info!("Backup: committed previously stuck ops file {:?}", committed);
        }
    }
}

fn load_seq(path: &PathBuf) -> Option<u64> {
    let content = fs::read_to_string(path).ok()?;
    content.trim().parse::<u64>().ok()
}

fn persist_seq(path: &PathBuf, seq: u64) {
    if let Err(e) = fs::write(path, seq.to_string()) {
        error!("Backup: failed to persist seq counter: {e}");
    }
}

// â”€â”€â”€ Snapshot / Restore â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Clone, serde::Serialize)]
pub struct SnapshotInfo {
    pub base_name: String,
    pub timestamp: String,
    pub total_size: u64,
    pub parts: Vec<PartInfo>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PartInfo {
    pub message_id: u64,
    pub filename: String,
    pub url: String,
    pub size: u64,
}

/// List available snapshots from the Discord backup thread.
pub async fn list_snapshots(
    discord: &dyn DiscordBackupBackend,
    thread_id: u64,
) -> Result<Vec<SnapshotInfo>, String> {
    use std::collections::HashMap;

    let atts = discord.list_backup_messages(thread_id, 50).await?;

    let mut snapshots: HashMap<String, Vec<PartInfo>> = HashMap::new();
    for att in &atts {
        let fname = &att.filename;
        if !fname.starts_with("backup_snapshot_") || !fname.ends_with(".zst") {
            continue;
        }
        let base = if let Some(pos) = fname.find(".part") {
            &fname[..pos]
        } else {
            &fname[..fname.len() - 4]
        };
        snapshots
            .entry(base.to_string())
            .or_default()
            .push(PartInfo {
                message_id: att.message_id,
                filename: fname.clone(),
                url: att.url.clone(),
                size: att.size,
            });
    }

    let mut result: Vec<SnapshotInfo> = snapshots
        .into_iter()
        .map(|(base_name, parts)| {
            let total_size: u64 = parts.iter().map(|p| p.size).sum();
            let ts = base_name
                .strip_prefix("backup_snapshot_")
                .unwrap_or(&base_name)
                .to_string();
            SnapshotInfo { base_name, timestamp: ts, total_size, parts }
        })
        .collect();
    result.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(result)
}

/// Download + decompress a snapshot and write to the target path.
pub async fn download_snapshot(
    discord: &dyn DiscordBackupBackend,
    snapshot: &SnapshotInfo,
) -> Result<(Vec<u8>, String), String> {
    let mut compressed = Vec::new();
    for part in &snapshot.parts {
        let data = discord.download_backup_attachment(&part.url).await?;
        compressed.extend_from_slice(&data);
    }
    let decompressed = zstd::decode_all(&compressed[..])
        .map_err(|e| format!("zstd decompression failed: {e}"))?;
    Ok((decompressed, format!("restored_{}.db", snapshot.timestamp)))
}

/// Replay ops files from the backup thread into a database connection.
pub async fn replay_ops<F>(
    discord: &dyn DiscordBackupBackend,
    apply_op: F,
    thread_id: u64,
    after_seq: u64,
) -> Result<u64, String>
where
    F: Fn(&Op) -> Result<(), String>,
{
    let atts = discord.list_backup_messages(thread_id, 100).await?;

    let ops_urls: Vec<String> = atts
        .iter()
        .filter(|a| a.filename.ends_with(".jsonl") && a.filename.starts_with("ops_"))
        .map(|a| a.url.clone())
        .collect();

    if ops_urls.is_empty() {
        return Ok(0);
    }

    let mut all_raw: Vec<Vec<u8>> = Vec::with_capacity(ops_urls.len());
    for url in &ops_urls {
        let data = discord.download_backup_attachment(url).await?;
        all_raw.push(data);
    }

    let mut all_ops: Vec<Op> = Vec::new();
    for raw in &all_raw {
        let content = String::from_utf8(raw.clone())
            .map_err(|e| format!("Invalid UTF-8 in ops file: {e}"))?;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() { continue; }
            if let Ok(op) = serde_json::from_str::<Op>(trimmed) {
                all_ops.push(op);
            }
        }
    }

    all_ops.sort_by_key(|op| op.seq());
    all_ops.retain(|op| op.seq() > after_seq);

    let mut prev: Option<u64> = None;
    for op in &all_ops {
        let s = op.seq();
        if let Some(p) = prev {
            if s != p + 1 {
                tracing::warn!("Backup restore: gap detected between seq {} and {}", p, s);
            }
        }
        prev = Some(s);
    }

    let replayed = all_ops.len() as u64;
    for op in &all_ops {
        if let Err(e) = apply_op(op) {
            return Err(e);
        }
    }
    Ok(replayed)
}

/// Core snapshot + upload logic. Creates a Discord backup thread,
/// uploads pre-prepared chunks with retry (3 attempts).
pub async fn run_snapshot_and_upload(
    discord: &dyn DiscordBackupBackend,
    chunks: Vec<(Vec<u8>, String)>,
    backup_service: &BackupService,
) -> Result<u64, String> {
    let thread_name = format!("db-{}", Utc::now().format("%Y%m%d-%H%M%S"));
    let new_thread_id = discord.create_backup_thread(&thread_name).await?;

    use futures_util::stream::{self, StreamExt};

    async fn upload_chunk(
        discord: &dyn DiscordBackupBackend,
        thread_id: u64,
        data: Vec<u8>,
        filename: String,
    ) {
        let mut last_err = String::new();
        for attempt in 1..=3 {
            match discord.upload_backup_file(thread_id, data.clone(), &filename).await {
                Ok(_) => return,
                Err(e) => {
                    last_err = format!("{e}");
                    warn!("Backup: upload {} failed (attempt {}/3): {e}", filename, attempt);
                }
            }
        }
        error!("Backup: upload {} failed after 3 retries: {last_err}", filename);
    }

    let tasks: Vec<_> = chunks.iter()
        .map(|(data, filename)| upload_chunk(discord, new_thread_id, data.clone(), filename.clone()))
        .collect();
    stream::iter(tasks).buffer_unordered(3).for_each(|_| async {}).await;

    let evicted = backup_service.push_backup_thread(new_thread_id);
    if let Some(old_id) = evicted {
        if let Err(e) = discord.delete_backup_thread(old_id).await {
            warn!("Backup: failed to delete old thread {}: {}", old_id, e);
        }
    }

    Ok(new_thread_id)
}


