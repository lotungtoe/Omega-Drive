use async_trait::async_trait;

#[async_trait]
pub trait FeatureLog: Send + Sync {
    fn log(&self, feature: &str, level: &str, message: &str);
    fn query(&self, feature: Option<&str>, limit: usize) -> Vec<String>;
}
