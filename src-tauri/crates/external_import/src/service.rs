use crate::downloader;

pub async fn get_metadata(url: String, cookies_browser: Option<String>) -> Result<downloader::Metadata, String> {
    downloader::get_metadata(&url, cookies_browser.as_deref()).await
}
