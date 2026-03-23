use std::sync::Arc;
use tokio::sync::{Semaphore, SemaphorePermit};

#[derive(Clone, Debug)]
pub struct RateLimiter {
    semaphore: Arc<Semaphore>,
}

pub struct RateLimitPermit<'a> {
    _permit: SemaphorePermit<'a>,
}
impl RateLimiter {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    pub async fn acquire(&self) -> RateLimitPermit<'_> {
        let permit = self.semaphore.acquire().await.unwrap();
        RateLimitPermit { _permit: permit }
    }
}
