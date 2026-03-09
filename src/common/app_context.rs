use indicatif::MultiProgress;
use reqwest::Client;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::common::progress::ProgressReporterMode;
use crate::config::Config;

#[derive(Clone)]
pub struct AppContext {
    pub config: Arc<Config>,
    pub http: Client,
    pub progress_reporter_mode: ProgressReporterMode,
    pub cancellation_token: CancellationToken,

    // TEMPORARY: Kept for backward compat during Step 1.1.
    // Will be removed in Step 1.3 when we migrate to progress_reporter_mode.
    pub multi_progress_bar: Arc<MultiProgress>,
}

impl AppContext {
    pub fn new(cfg: Arc<Config>) -> Self {
        Self {
            config: cfg,
            http: Client::new(),
            progress_reporter_mode: ProgressReporterMode::Noop,
            cancellation_token: CancellationToken::new(),
            multi_progress_bar: Arc::new(MultiProgress::new()),
        }
    }
}
