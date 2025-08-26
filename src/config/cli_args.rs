use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct CliArgs {
    #[arg(long)]
    pub pg_host: Option<String>,
    #[arg(long)]
    pub pg_user: Option<String>,
    #[arg(long)]
    pub pg_pass: Option<String>,
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
    pub reset_db: Option<bool>,
    #[arg(long)]
    pub retry_interval_ms: Option<u64>,
    #[arg(long)]
    pub retry_max_interval_ms: Option<u64>,
    #[arg(long)]
    pub retries: Option<usize>,
}
