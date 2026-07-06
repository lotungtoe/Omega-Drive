use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{error::Error as StdError, fmt};

use crate::core::error_codes as codes;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub context: Option<Value>,
    pub retryable: bool,
    pub source: Option<String>,
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            context: None,
            retryable: false,
            source: None,
        }
    }

    pub fn with_context(mut self, context: Value) -> Self {
        self.context = Some(context);
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn retryable(mut self, retryable: bool) -> Self {
        self.retryable = retryable;
        self
    }

    pub fn feature_disabled(feature: &str) -> Self {
        AppError::new(
            codes::E_FEATURE_DISABLED,
            format!("Feature '{feature}' is disabled"),
        )
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl StdError for AppError {}

pub fn report(_feature: &str, err: AppError) -> AppError { err }

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
    report(
        feature,
        AppError::new(code, message)
            .with_context(context)
            .with_source(err_any.to_string()),
    )
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        match err.downcast::<AppError>() {
            Ok(app_err) => app_err,
            Err(err) => {
                AppError::new(codes::E_UNKNOWN, "Unhandled error").with_source(err.to_string())
            }
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::new(codes::E_JSON, "JSON error").with_source(err.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::new(codes::E_IO, "I/O error").with_source(err.to_string())
    }
}

impl From<String> for AppError {
    fn from(err: String) -> Self {
        AppError::new(codes::E_UNKNOWN, err)
    }
}

impl From<&str> for AppError {
    fn from(err: &str) -> Self {
        AppError::new(codes::E_UNKNOWN, err)
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        AppError::new(codes::E_DB, "Database error").with_source(err.to_string())
    }
}
