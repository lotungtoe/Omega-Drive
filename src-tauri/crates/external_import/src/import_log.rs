use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncRead, ReadBuf};

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn log_path(base_dir: &Path) -> PathBuf {
    let dir = base_dir.join("logs");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("import.log")
}

fn should_log() -> bool {
    match std::env::var("DEBUG") {
        Ok(v) if v == "1" => true,
        _ => false,
    }
}

pub fn log_event(
    base_dir: &Path,
    session_id: &str,
    url: &str,
    phase: &str,
    duration_ms: u64,
    bytes: Option<u64>,
    error: Option<&str>,
) {
    if !should_log() { return; }
    let path = log_path(base_dir);
    let mut file = match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[import_log] Failed to open log file {}: {}", path.display(), e);
            return;
        }
    };
    let entry = serde_json::json!({
        "ts": unix_ms(),
        "session": session_id,
        "url": url,
        "phase": phase,
        "ms": duration_ms,
        "bytes": bytes,
        "error": error,
    });
    let _ = writeln!(file, "{entry}");
}

pub struct MeteredStream<R> {
    pub inner: R,
    pub bytes_read: Arc<AtomicU64>,
}

impl<R: AsyncRead + Unpin> AsyncRead for MeteredStream<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let before = buf.filled().len();
        let poll = Pin::new(&mut self.inner).poll_read(cx, buf);
        let after = buf.filled().len();
        self.bytes_read.fetch_add((after - before) as u64, Ordering::Relaxed);
        poll
    }
}

pub async fn monitor_upload_progress(
    base_dir: PathBuf,
    session_id: String,
    url: String,
    bytes_read: Arc<AtomicU64>,
    total_bytes: u64,
    interval_ms: u64,
) {
    if !should_log() { return; }
    let t0 = SystemTime::now();
    let mut logged = false;
    loop {
        tokio::time::sleep(Duration::from_millis(interval_ms)).await;
        let read = bytes_read.load(Ordering::Relaxed);
        if read == 0 && logged {
            continue;
        }
        let elapsed = t0.elapsed().unwrap_or_default().as_millis() as u64;
        log_event(&base_dir, &session_id, &url, "upload_progress", elapsed, Some(read), None);
        logged = true;
        if read >= total_bytes {
            break;
        }
    }
}
