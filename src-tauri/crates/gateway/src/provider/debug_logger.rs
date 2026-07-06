pub trait DebugLogger: Send + Sync {
    fn debug_write(&self, channel: &str, message: &str);
    fn debug_core_init(&self, base_dir: &str);
}
