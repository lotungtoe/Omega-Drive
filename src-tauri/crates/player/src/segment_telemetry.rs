use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SegmentFetchSample {
    pub provider: String,
    pub bytes: u64,
    pub ttfb_ms: u64,
    pub total_ms: u64,
    pub retries: u32,
    pub ok: bool,
}

#[derive(Clone)]
pub struct SegmentTelemetry {
    limit: usize,
    inner: Arc<Mutex<VecDeque<SegmentFetchSample>>>,
}

impl SegmentTelemetry {
    pub fn new(limit: usize) -> Self {
        Self {
            limit: limit.max(1),
            inner: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn record(&self, sample: SegmentFetchSample) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.push_front(sample);
        while inner.len() > self.limit {
            inner.pop_back();
        }
    }

    pub fn recommended_parallelism(&self, provider: &str, requested: usize) -> usize {
        let requested = requested.max(1);
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let recent: Vec<&SegmentFetchSample> = inner
            .iter()
            .filter(|sample| sample.provider == provider)
            .take(8)
            .collect();

        if recent.len() < 4 {
            return requested;
        }

        let failures = recent.iter().filter(|sample| !sample.ok).count();
        let retries: u32 = recent.iter().map(|sample| sample.retries).sum();
        let avg_ttfb_ms =
            recent.iter().map(|sample| sample.ttfb_ms).sum::<u64>() / recent.len() as u64;

        if failures >= 2 || retries >= recent.len() as u32 || avg_ttfb_ms > 1_500 {
            1
        } else if failures >= 1 || retries > 0 || avg_ttfb_ms > 800 {
            requested.min(2)
        } else {
            requested
        }
    }
}

impl Default for SegmentTelemetry {
    fn default() -> Self {
        Self::new(256)
    }
}


