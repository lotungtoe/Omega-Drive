use std::fmt;

use crate::core::error::AppError;
use crate::core::error_codes;

pub type UploadResult<T> = Result<T, UploadError>;

#[derive(Debug, Clone)]
pub enum UploadError {
    Validation { message: String },
    Conflict { message: String },
    Db { message: String, source: Option<String> },
    Io { message: String, source: Option<String> },
    Provider { message: String, source: Option<String> },
    Timeout { message: String },
    Internal { message: String, source: Option<String> },
}

impl UploadError {
    pub fn create_validation_error(message: impl Into<String>) -> Self {
        Self::Validation { message: message.into() }
    }
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict { message: message.into() }
    }
    pub fn db(message: impl Into<String>, err: impl ToString) -> Self {
        Self::Db { message: message.into(), source: Some(err.to_string()) }
    }
    pub fn provider(message: impl Into<String>, err: impl ToString) -> Self {
        Self::Provider { message: message.into(), source: Some(err.to_string()) }
    }
    pub fn provider_message(message: impl Into<String>) -> Self {
        Self::Provider { message: message.into(), source: None }
    }
    pub fn io(message: impl Into<String>, err: impl ToString) -> Self {
        Self::Io { message: message.into(), source: Some(err.to_string()) }
    }
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::Timeout { message: message.into() }
    }
    pub fn internal(message: impl Into<String>, err: impl ToString) -> Self {
        Self::Internal { message: message.into(), source: Some(err.to_string()) }
    }
}

impl fmt::Display for UploadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation { message } | Self::Conflict { message }
            | Self::Db { message, .. } | Self::Io { message, .. }
            | Self::Provider { message, .. } | Self::Timeout { message }
            | Self::Internal { message, .. } => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for UploadError {}

impl From<UploadError> for AppError {
    fn from(err: UploadError) -> Self {
        match err {
            UploadError::Validation { message } => AppError::new(error_codes::E_INVALID_INPUT, message),
            UploadError::Conflict { message } => AppError::new(error_codes::E_UPLOAD_CONFLICT, message),
            UploadError::Db { message, source } => {
                let mut err = AppError::new(error_codes::E_DB, message);
                if let Some(source) = source { err = err.with_source(source); }
                err
            }
            UploadError::Io { message, source } => {
                let mut err = AppError::new(error_codes::E_IO, message);
                if let Some(source) = source { err = err.with_source(source); }
                err
            }
            UploadError::Provider { message, source } => {
                let mut err = AppError::new(error_codes::E_UNAVAILABLE, message);
                if let Some(source) = source { err = err.with_source(source); }
                err
            }
            UploadError::Timeout { message } => AppError::new(error_codes::E_TIMEOUT, message).retryable(true),
            UploadError::Internal { message, source } => {
                let mut err = AppError::new(error_codes::E_UPLOAD_FAILED, message);
                if let Some(source) = source { err = err.with_source(source); }
                err
            }
        }
    }
}
