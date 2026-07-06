use std::time::{Duration, Instant};

const DEFAULT_BURST_SECS: f64 = 2.0;

pub(crate) struct TokenBucket {
    rate_bps: f64,
    capacity: f64,
    tokens: f64,
    last_refill: Instant,
}

impl TokenBucket {
    pub(crate) fn new(rate_bps: f64) -> Self {
        let now = Instant::now();
        let capacity = if rate_bps > 0.0 { rate_bps * DEFAULT_BURST_SECS } else { 0.0 };
        Self { rate_bps, capacity, tokens: capacity, last_refill: now }
    }

    pub(crate) fn set_rate(&mut self, rate_bps: f64) {
        self.rate_bps = rate_bps;
        self.capacity = if rate_bps > 0.0 { rate_bps * DEFAULT_BURST_SECS } else { 0.0 };
        if self.capacity > 0.0 { self.tokens = self.tokens.min(self.capacity); } else { self.tokens = 0.0; }
        self.last_refill = Instant::now();
    }

    fn refill(&mut self) {
        if self.rate_bps <= 0.0 { return; }
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        if elapsed <= 0.0 { return; }
        self.tokens = (self.tokens + elapsed * self.rate_bps).min(self.capacity);
        self.last_refill = now;
    }

    pub(crate) async fn acquire(&mut self, bytes: usize) {
        if self.rate_bps <= 0.0 { return; }
        self.refill();
        let needed = bytes as f64;
        if self.tokens >= needed { self.tokens -= needed; return; }
        let deficit = needed - self.tokens;
        let wait_secs = deficit / self.rate_bps;
        self.tokens = 0.0;
        if wait_secs > 0.0 { tokio::time::sleep(Duration::from_secs_f64(wait_secs)).await; }
    }
}

pub(crate) struct DownloadThrottle {
    bucket: TokenBucket,
    ema_bps: f64,
}

impl DownloadThrottle {
    pub(crate) fn new(rate_bps: f64) -> Self { Self { bucket: TokenBucket::new(rate_bps), ema_bps: 0.0 } }
    pub(crate) fn set_rate(&mut self, rate_bps: f64) { self.bucket.set_rate(rate_bps); }
    pub(crate) async fn throttle(&mut self, bytes: usize) { self.bucket.acquire(bytes).await; }
    pub(crate) fn observe(&mut self, bytes: usize, elapsed: Duration) {
        let secs = elapsed.as_secs_f64();
        if secs <= 0.0 { return; }
        let inst_bps = bytes as f64 / secs;
        self.ema_bps = if self.ema_bps <= 0.0 { inst_bps } else { self.ema_bps * 0.7 + inst_bps * 0.3 };
    }
    pub(crate) fn ema_bps(&self) -> f64 { self.ema_bps }
}
