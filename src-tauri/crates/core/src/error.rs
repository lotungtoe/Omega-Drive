pub use omega_drive_gateway::core::error::*;

pub fn log_app_error(feature: &str, err: &AppError) {
    let target = match feature {
        "diagnostics" => "feature::diagnostics",
        "discord" => "feature::discord",
        "download" => "feature::download",
        "drive" => "feature::drive",
        "extensions" => "feature::extensions",
        "onboarding" => "feature::onboarding",
        "player" => "feature::player",
        "settings" => "feature::settings",
        "tenant" => "feature::tenant",
        "upload" => "feature::upload",
        _ => "feature::unknown",
    };
    tracing::error!(target, code = %err.code, message = %err.message, context = ?err.context, source = ?err.source, retryable = err.retryable);
}

pub fn report(feature: &str, err: AppError) -> AppError {
    log_app_error(feature, &err);
    err
}

pub fn wrap_error(
    feature: &str,
    code: &str,
    message: impl Into<String>,
    context: serde_json::Value,
    err: impl Into<anyhow::Error>,
) -> AppError {
    let err_any: anyhow::Error = err.into();
    if let Some(app_err) = err_any.downcast_ref::<AppError>() {
        return report(feature, app_err.clone());
    }
    report(feature, AppError::new(code, message).with_context(context).with_source(err_any.to_string()))
}
