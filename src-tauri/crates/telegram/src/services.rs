use std::sync::OnceLock;

use omega_drive_gateway::provider::debug_logger::DebugLogger;

static LOGGER: OnceLock<Box<dyn DebugLogger>> = OnceLock::new();

pub fn init(logger: Box<dyn DebugLogger>) {
    LOGGER.set(logger).ok();
}

fn logger() -> &'static dyn DebugLogger {
    LOGGER.get().map(|b| &**b).unwrap_or(&NOOP)
}

struct NoopLogger;
impl DebugLogger for NoopLogger {
    fn debug_write(&self, _channel: &str, _message: &str) {}
    fn debug_core_init(&self, _base_dir: &str) {}
}

static NOOP: NoopLogger = NoopLogger;

pub fn debug_write(channel: &str, message: &str) {
    logger().debug_write(channel, message);
}
