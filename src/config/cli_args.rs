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
    pub season_limit: Option<usize>,
    #[arg(long)]
    pub max_db_connections: Option<u32>,
    #[arg(long)]
    pub upsert_concurrency: Option<usize>,
    #[arg(long)]
    pub reset_db: Option<bool>,
    #[arg(long)]
    pub retry_jitter_duration_ms: Option<u64>,
    #[arg(long)]
    pub retries: Option<usize>,
}
