use futures::lock::Mutex;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{Semaphore, SemaphorePermit};

#[derive(Clone, Debug)]
pub struct RateLimiter {
    semaphore: Arc<Semaphore>,
    last_request: Arc<Mutex<Instant>>,
    min_spacing: Duration,
}

pub struct RateLimitPermit<'a> {
    _permit: SemaphorePermit<'a>,
}
impl RateLimiter {
    pub fn new(max_concurrent: usize, min_spacing_ms: u64) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            last_request: Arc::new(Mutex::new(Instant::now())),
            min_spacing: Duration::from_millis(min_spacing_ms),
        }
    }

    pub async fn acquire(&self) -> RateLimitPermit<'_> {
        let permit = self.semaphore.acquire().await.unwrap();

        let mut last = self.last_request.lock().await;
        let elapsed = last.elapsed();
        if elapsed < self.min_spacing {
            tokio::time::sleep(self.min_spacing - elapsed).await;
        }
        *last = Instant::now();

        RateLimitPermit { _permit: permit }
    }
}
