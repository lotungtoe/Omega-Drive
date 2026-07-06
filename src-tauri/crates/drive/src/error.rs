use omega_drive_gateway::core::error::AppError;

#[derive(Debug, Clone)]
pub enum DriveError {
    Validation(String),
    NotFound(String),
    Db(String, String),
    Provider(String, String),
    Internal(String, String),
    Io(String, String),
}

impl DriveError {
    pub fn validation(msg: impl Into<String>) -> Self { Self::Validation(msg.into()) }
    pub fn not_found(msg: impl Into<String>) -> Self { Self::NotFound(msg.into()) }
    pub fn db(msg: impl Into<String>, source: impl std::fmt::Display) -> Self { Self::Db(msg.into(), source.to_string()) }
    pub fn provider(msg: impl Into<String>, source: impl std::fmt::Display) -> Self { Self::Provider(msg.into(), source.to_string()) }
    pub fn internal(msg: impl Into<String>, source: impl std::fmt::Display) -> Self { Self::Internal(msg.into(), source.to_string()) }
    pub fn io(msg: impl Into<String>, source: impl std::fmt::Display) -> Self { Self::Io(msg.into(), source.to_string()) }
}

impl From<DriveError> for AppError {
    fn from(e: DriveError) -> Self {
        match e {
            DriveError::Validation(m) => AppError::new("drive_validation", m),
            DriveError::NotFound(m) => AppError::new("drive_not_found", m),
            DriveError::Db(m, s) => AppError::new("drive_db", m).with_source(s),
            DriveError::Provider(m, s) => AppError::new("drive_provider", m).with_source(s),
            DriveError::Internal(m, s) => AppError::new("drive_internal", m).with_source(s),
            DriveError::Io(m, s) => AppError::new("drive_io", m).with_source(s),
        }
    }
}
