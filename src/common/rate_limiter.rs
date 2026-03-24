use futures::lock::Mutex;
use std::{
    sync::{
        atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::sync::{Semaphore, SemaphorePermit};

#[derive(Clone, Debug)]
pub struct RateLimiterConfig {
    pub min_permits: usize,
    pub max_permits: usize,

    pub min_spacing_us: u64,
    pub max_spacing_us: u64,
}

#[derive(Clone, Debug)]
pub struct RateLimiter {
    semaphore: Arc<Semaphore>,

    last_request: Arc<Mutex<Instant>>,
    success_count: Arc<AtomicU32>,
    consecutive_429s: Arc<AtomicU32>,

    current_spacing_us: Arc<AtomicU64>,
    current_permits: Arc<AtomicUsize>,

    config: RateLimiterConfig,
}

pub struct RateLimitPermit<'a> {
    _permit: SemaphorePermit<'a>,
}
impl RateLimiter {
    pub fn new(config: RateLimiterConfig) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(config.min_permits)),
            last_request: Arc::new(Mutex::new(Instant::now())),
            success_count: Arc::new(AtomicU32::new(0)),
            consecutive_429s: Arc::new(AtomicU32::new(0)),
            current_spacing_us: Arc::new(AtomicU64::new(config.max_spacing_us)),
            current_permits: Arc::new(AtomicUsize::new(config.min_permits)),
            config,
        }
    }

    pub async fn acquire(&self) -> RateLimitPermit<'_> {
        let permit = self.semaphore.acquire().await.unwrap();

        let mut last = self.last_request.lock().await;
        let current_spacing_us = self.current_spacing_us.load(Ordering::Relaxed);
        let current_spacing = Duration::from_micros(current_spacing_us);

        let elapsed = last.elapsed();
        if elapsed < current_spacing {
            tokio::time::sleep(current_spacing - elapsed).await;
        }
        *last = Instant::now();

        RateLimitPermit { _permit: permit }
    }

    pub fn on_success(&self) {
        self.consecutive_429s.store(0, Ordering::Relaxed);

        let count = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;

        if count % 50 == 0 {
            let old_permits = self.current_permits.load(Ordering::Relaxed);
            let old_spacing = self.current_spacing_us.load(Ordering::Relaxed);

            let mut changed = false;

            if old_permits < self.config.max_permits {
                self.semaphore.add_permits(1);
                self.current_permits
                    .store(old_permits + 1, Ordering::Relaxed);
                changed = true;
            }

            // MIMD for spacing: multiply by 0.9 to speed up by 10%
            let new_spacing = ((old_spacing as f64) * 0.9) as u64;
            let new_spacing = new_spacing.max(self.config.min_spacing_us);

            if new_spacing != old_spacing {
                self.current_spacing_us
                    .store(new_spacing, Ordering::Relaxed);
                changed = true;
            }

            if changed {
                let current_permits = self.current_permits.load(Ordering::Relaxed);
                let current_spacing_ms = new_spacing / 1000;
                tracing::info!(
                    permits = current_permits,
                    spacing_ms = current_spacing_ms,
                    success_count = count,
                    "Rate limiter speeding up after 50 successes (total: {})",
                    count
                );
            }
        }
    }

    pub async fn on_rate_limited(&self) {
        let consecutive = self.consecutive_429s.fetch_add(1, Ordering::Relaxed) + 1;
        self.success_count.store(0, Ordering::Relaxed);

        let current_permits = self.current_permits.load(Ordering::Relaxed);
        let new_permits = if consecutive > 3 {
            self.config.min_permits
        } else {
            (current_permits / 2).max(self.config.min_permits)
        };
        self.current_permits.store(new_permits, Ordering::Relaxed);

        let permits_to_remove = current_permits.saturating_sub(new_permits);
        for _ in 0..permits_to_remove {
            let sem = self.semaphore.clone();
            tokio::spawn(async move {
                let permit = sem.acquire().await.unwrap();
                permit.forget();
            });
        }

        let old_spacing = self.current_spacing_us.load(Ordering::Relaxed);
        let new_spacing = ((old_spacing as f64) * 1.5) as u64;
        let new_spacing = new_spacing.min(self.config.max_spacing_us);
        self.current_spacing_us
            .store(new_spacing, Ordering::Relaxed);

        let new_spacing_ms = new_spacing / 1000;

        // Circuit breaker: if we're getting repeated 429s, pause exponentially
        // This gives API ban windows time to expire
        if consecutive > 3 {
            let pause_seconds = 2_u64.pow((consecutive - 3).min(8)).min(300);
            tracing::warn!(
                permits = new_permits,
                spacing_ms = new_spacing_ms,
                consecutive_429s = consecutive,
                pause_seconds = pause_seconds,
                "Circuit breaker activated! Pausing {} seconds after {} consecutive 429s",
                pause_seconds,
                consecutive
            );
            tokio::time::sleep(Duration::from_secs(pause_seconds)).await;
        } else {
            tracing::warn!(
                permits = new_permits,
                spacing_ms = new_spacing_ms,
                consecutive_429s = consecutive,
                "Rate limited! Backing off aggressively"
            );
        }
    }
}
