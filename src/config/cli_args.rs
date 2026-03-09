use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(long)]
    pub database_url: Option<String>,
    #[arg(long)]
    pub api_concurrency_limit: Option<usize>,
    #[arg(long)]
    pub max_db_connections: Option<u32>,
    #[arg(long)]
    pub db_concurrency_limit: Option<usize>,
    #[arg(long)]
    pub db_query_batch_size: Option<usize>,
    #[arg(long)]
    pub db_query_batch_timeout_ms: Option<u64>,
    #[arg(long)]
    pub retry_interval_ms: Option<u64>,
    #[arg(long)]
    pub retry_max_interval_ms: Option<u64>,
    #[arg(long)]
    pub retries: Option<usize>,
    #[arg(long)]
    pub progress_bar_style_format: Option<String>,
    #[arg(long)]
    pub progress_spinner_style_format: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Scrape data from various sources
    Scrape {
        #[command(subcommand)]
        source: ScrapeSource,
    },
}

#[derive(Subcommand, Debug)]
pub enum ScrapeSource {
    /// Scrape NHL data
    Nhl {
        /// Reset the database before scraping (debug builds only)
        #[cfg(debug_assertions)]
        #[arg(long)]
        reset: bool,
    },
}
