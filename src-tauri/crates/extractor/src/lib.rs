use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct KreuzbergResult {
    pub content: String,
    pub mime_type: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug)]
pub enum KreuzbergError {
    ExtractionFailed(String),
}

impl std::fmt::Display for KreuzbergError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExtractionFailed(msg) => write!(f, "extraction failed: {}", msg),
        }
    }
}

pub async fn extract_text(bytes: &[u8], _ext: &str) -> Result<KreuzbergResult, KreuzbergError> {
    let config = kreuzberg::ExtractionConfig::default();
    let result = kreuzberg::extract_bytes(bytes, "", &config)
        .await
        .map_err(|e| KreuzbergError::ExtractionFailed(e.to_string()))?;

    let metadata = serde_json::to_value(&result.metadata)
        .unwrap_or(serde_json::Value::Null);

    Ok(KreuzbergResult {
        content: result.content,
        mime_type: result.mime_type.into_owned(),
        metadata,
    })
}
