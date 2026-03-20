use futures::stream::{self, StreamExt};
use indicatif::{MultiProgress, ProgressStyle};
use reqwest::Client;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

use crate::{
    common::{
        data_source::DataSource,
        progress::{ProgressReporter, ProgressReporterMode},
    },
    config::Config,
};

#[derive(Clone)]
pub struct AppContext {
    pub config: Arc<Config>,
    pub http: Client,
    pub progress_reporter_mode: ProgressReporterMode,
    current_progress: Arc<Mutex<Option<Box<dyn ProgressReporter + Send>>>>,
    pub cancellation_token: CancellationToken,
    pub sources: Arc<Vec<Arc<dyn DataSource>>>,
}

impl AppContext {
    pub fn new(cfg: Arc<Config>, mp: MultiProgress, disable_progress: bool) -> Self {
        let progress_reporter_mode = if !disable_progress {
            let bar_style = ProgressStyle::default_bar()
                .template(&cfg.progress_bar_style_format)
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .progress_chars("##-");
            let spinner_style = ProgressStyle::default_spinner()
                .template(&cfg.progress_spinner_style_format)
                .unwrap_or_else(|_| ProgressStyle::default_spinner());
            ProgressReporterMode::Indicatif {
                mp: Arc::new(mp),
                bar_style,
                spinner_style,
            }
        } else {
            ProgressReporterMode::Noop
        };

        Self {
            config: cfg,
            http: Client::new(),
            progress_reporter_mode,
            current_progress: Arc::new(Mutex::new(None)),
            cancellation_token: CancellationToken::new(),
            sources: Arc::new(Vec::new()),
        }
    }

    pub fn with_sources(mut self, sources: Vec<Arc<dyn DataSource>>) -> Self {
        self.sources = Arc::new(sources);
        self
    }

    pub fn init_progress(&self, total: Option<usize>, msg: &str) {
        let pb = self.progress_reporter_mode.create_reporter(total, msg);
        *self.current_progress.lock().unwrap() = Some(pb);
    }

    pub fn inc_progress(&self, n: usize) {
        if let Some(pb) = self.current_progress.lock().unwrap().as_ref() {
            pb.inc(n);
        }
    }

    pub fn finish_progress(&self) {
        if let Some(pb) = self.current_progress.lock().unwrap().take() {
            pb.finish();
        }
    }

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
