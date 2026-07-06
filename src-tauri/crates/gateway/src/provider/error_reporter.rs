use crate::core::error::AppError;

pub trait ErrorReporter: Send + Sync {
    fn report(&self, feature: &str, err: AppError) -> AppError;

    fn wrap_error(
        &self,
        feature: &str,
        code: &str,
        message: impl Into<String>,
        context: serde_json::Value,
        err: impl Into<anyhow::Error>,
    ) -> AppError;
}
