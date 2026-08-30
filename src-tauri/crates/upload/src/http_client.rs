use std::sync::RwLock;
use std::time::Duration;

static CLIENT: RwLock<Option<reqwest::Client>> = RwLock::new(None);

pub fn http_client() -> &'static reqwest::Client {
    {
        let guard = CLIENT.read().expect("CLIENT poisoned");
        if let Some(c) = guard.as_ref() {
            return c;
        }
    }
    rebuild_http_client(600)
}

pub fn rebuild_http_client(keep_alive_s: u64) -> &'static reqwest::Client {
    let new_client = reqwest::Client::builder()
        .tcp_keepalive(Duration::from_secs(keep_alive_s))
        .build()
        .expect("Failed to build reqwest client");
    let mut guard = CLIENT.write().expect("CLIENT poisoned");
    *guard = Some(new_client);
    guard.as_ref().expect("just inserted")
}
