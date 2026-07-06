use std::sync::{Mutex, OnceLock};
use tracing::info;

pub(crate) static DEBUG_FILE: OnceLock<Mutex<std::fs::File>> = OnceLock::new();

pub fn debug_init(base_dir: &std::path::Path) {
    let log_dir = base_dir.join("logs");
    let _ = std::fs::create_dir_all(&log_dir);
    let path = log_dir.join("bridge_debug.log");
    let _ = std::fs::remove_file(&path);
    match std::fs::OpenOptions::new().append(true).create(true).open(&path) {
        Ok(f) => {
            let _ = DEBUG_FILE.set(Mutex::new(f));
        },
        Err(e) => { info!("[debug_log] Failed to open {}: {}", path.display(), e); }
    }
}



#[macro_export]
macro_rules! debug_log {
    ($tag:expr, $($arg:tt)+) => {
        if std::env::var("DEBUG").is_ok() {
            if let Some(mutex) = $crate::debug::DEBUG_FILE.get() {
                if let Ok(mut f) = mutex.lock() {
                    use std::io::Write;
                    let _ = writeln!(f, "[{}] {}: {}", chrono::Utc::now(), $tag, format_args!($($arg)+));
                    let _ = f.flush();
                }
            }
            tracing::info!("[{}] {}", $tag, format_args!($($arg)+));
        }
    };
}


