use futures::stream::{self, StreamExt};
use indicatif::{MultiProgress, ProgressStyle};
use reqwest::Client;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::common::data_source::DataSource;
use crate::common::progress::{ProgressReporter, ProgressReporterMode};
use crate::config::Config;

#[derive(Clone)]
pub struct AppContext {
    pub config: Arc<Config>,
    pub http: Client,
    pub progress_reporter_mode: ProgressReporterMode,
    pub cancellation_token: CancellationToken,
    pub sources: Arc<Vec<Arc<dyn DataSource>>>,
}

impl AppContext {
    pub fn new(cfg: Arc<Config>, disable_progress: bool) -> Self {
        let progress_reporter_mode = if !disable_progress {
            let mp = Arc::new(MultiProgress::new());
            let style = ProgressStyle::default_bar()
                .template(&cfg.progress_bar_style_format)
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .progress_chars("##-");
            ProgressReporterMode::Indicatif(mp, style)
        } else {
            ProgressReporterMode::Noop
        };

        Self {
            config: cfg,
            http: Client::new(),
            progress_reporter_mode,
            cancellation_token: CancellationToken::new(),
            sources: Arc::new(Vec::new()),
        }
    }

    pub fn with_sources(mut self, sources: Vec<Arc<dyn DataSource>>) -> Self {
        self.sources = Arc::new(sources);
        self
    }

    pub fn with_progress_bar<F, R>(&self, total: u64, msg: &str, f: F) -> R
    where
        F: FnOnce(&dyn ProgressReporter) -> R,
    {
        let pb = self
            .progress_reporter_mode
            .create_reporter(Some(total), msg);
        let result = f(&*pb);
        pb.finish();
        result
    }

    // Spinner without known total
    pub fn with_spinner<F, R>(&self, msg: &str, f: F) -> R
    where
        F: FnOnce(&dyn ProgressReporter) -> R,
    {
        let pb = self.progress_reporter_mode.create_reporter(None, msg);
        let result = f(&*pb);
        pb.finish();
        result
    }

    // Note: No async variants provided. For async progress reporting with complex
    // lifetime requirements (streams, multiple awaits), use explicit calls:
    //   let pb = app_context.progress_reporter_mode.create_reporter(...);
    //   let result = async_work_with_pb().await;
    //   pb.finish();

    /// Execute futures concurrently with respect to configured DB concurrency limit.
    /// Respects cancellation token for graceful shutdown.
    pub async fn with_db_concurrency<Fut, T>(&self, futures: Vec<Fut>) -> Vec<T>
    where
        Fut: std::future::Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        stream::iter(futures)
            .buffer_unordered(self.config.db_concurrency_limit)
            .collect::<Vec<_>>()
            .await
    }
}
