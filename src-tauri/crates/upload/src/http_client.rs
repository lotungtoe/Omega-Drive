use std::sync::OnceLock;
use std::time::Duration;

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

pub fn init_http_client(keep_alive_s: u64) {
    let _ = CLIENT.set(
        reqwest::Client::builder()
            .tcp_keepalive(Duration::from_secs(keep_alive_s))
            .build()
            .expect("Failed to build reqwest client"),
    );
}

pub fn http_client() -> &'static reqwest::Client {
    CLIENT
        .get()
        .expect("http_client not initialized; call init_http_client first")
}
