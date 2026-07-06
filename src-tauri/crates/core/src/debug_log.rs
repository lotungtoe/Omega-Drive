use std::sync::{Mutex, OnceLock};

static CORE_DEBUG_FILE: OnceLock<Mutex<std::fs::File>> = OnceLock::new();

pub fn debug_core_init(base_dir: &std::path::Path) {
    if std::env::var("DEBUG").is_err() {
        return;
    }
    let log_dir = base_dir.join("logs");
    let _ = std::fs::create_dir_all(&log_dir);
    let path = log_dir.join("bridge_debug.log");
    if let Ok(f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = CORE_DEBUG_FILE.set(Mutex::new(f));
    }
}

pub fn debug_write(tag: &str, msg: &str) {
    if let Some(m) = CORE_DEBUG_FILE.get() {
        if let Ok(mut f) = m.lock() {
            use std::io::Write;
            let _ = writeln!(f, "[{}] {}: {}", chrono::Utc::now(), tag, msg);
            let _ = f.flush();
        }
    }
}
