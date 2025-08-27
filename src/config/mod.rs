pub mod cli_args;
pub mod env_vars;

use clap::Parser;
use indicatif::ProgressStyle;
use once_cell::sync::Lazy;

use cli_args::CliArgs;
use env_vars::EnvironmentVariables;

const DEFAULT_API_CONCURRENCY_LIMIT: usize = 32;
const DEFAULT_MAX_DB_CONNECTIONS: u32 = 100;
const DEFAULT_DB_CONCURRENCY_LIMIT: usize = 50;
const DEFAULT_DB_QUERY_BATCH_SIZE: usize = 1_000;
const DEFAULT_DB_QUERY_BATCH_TIMEOUT_MS: u64 = 100;
const DEFAULT_RETRY_INTERVAL_MS: u64 = 100;
const DEFAULT_RETRY_MAX_INTERVAL_MS: u64 = 10_000;
const DEFAULT_RETRIES: usize = 5;

pub static CONFIG: Lazy<Config> = Lazy::new(|| Config::from_env_and_args());

pub struct Config {
    pub pg_host: String,
    pub pg_user: String,
    pub pg_pass: String,
    pub api_concurrency_limit: usize,
    pub max_db_connections: u32,
    pub db_concurrency_limit: usize,
    pub db_query_batch_size: usize,
    pub db_query_batch_timeout_ms: u64,
    pub reset_db: bool,
    pub retry_interval_ms: u64,
    pub retry_max_interval_ms: u64,
    pub retries: usize,
    pub progress_bar_style: ProgressStyle,
    pub spinner_style: ProgressStyle,
}

impl Config {
    #[tracing::instrument]
    pub fn from_env_and_args() -> Self {
        let cli_args: CliArgs = CliArgs::parse();
        let env_vars: EnvironmentVariables = EnvironmentVariables::from_env();

        let pg_host: String = cli_args.pg_host.or(env_vars.pg_host).unwrap_or_else(|| {
            panic!(
                r#"Failed to load required environment variable: `PG_HOST`
                At minimum, `PG_HOST`, `PG_USER`, and `PG_PASS` must be defined 1) as environment variables, 2) in a file named `.env`, or 3) as command line arguments.
                Panicking!"#
            )
        });
        let pg_user: String = cli_args.pg_user.or(env_vars.pg_user).unwrap_or_else(|| {
            panic!(
                r#"Failed to load required environment variable: `PG_USER`
                At minimum, `PG_HOST`, `PG_USER`, and `PG_PASS` must be defined 1) as environment variables, 2) in a file named `.env`, or 3) as command line arguments.
                Panicking!"#
            )
        });
        let pg_pass: String = cli_args.pg_pass.or(env_vars.pg_pass).unwrap_or_else(|| {
            panic!(
                r#"Failed to load required environment variable: `PG_PASS`
                At minimum, `PG_HOST`, `PG_USER`, and `PG_PASS` must be defined 1) as environment variables, 2) in a file named `.env`, or 3) as command line arguments.
                Panicking!"#
            )
        });

        let api_concurrency_limit: usize = cli_args
            .api_concurrency_limit
            .or(env_vars.api_concurrency_limit)
            .unwrap_or(DEFAULT_API_CONCURRENCY_LIMIT);

        let max_db_connections: u32 = cli_args
            .max_db_connections
            .or(env_vars.max_db_connections)
            .unwrap_or(DEFAULT_MAX_DB_CONNECTIONS);

        let db_concurrency_limit: usize = cli_args
            .db_concurrency_limit
            .or(env_vars.db_concurrency_limit)
            .unwrap_or(DEFAULT_DB_CONCURRENCY_LIMIT);

        let db_query_batch_size: usize = cli_args
            .db_query_batch_size
            .or(env_vars.db_query_batch_size)
            .unwrap_or(DEFAULT_DB_QUERY_BATCH_SIZE);

        let db_query_batch_timeout_ms = cli_args
            .db_query_batch_timeout_ms
            .or(env_vars.db_query_batch_timeout_ms)
            .unwrap_or(DEFAULT_DB_QUERY_BATCH_TIMEOUT_MS);

        let reset_db: bool = cli_args.reset_db.or(env_vars.reset_db).unwrap_or(false);

        let retry_interval_ms = cli_args
            .retry_interval_ms
            .or(env_vars.retry_interval_ms)
            .unwrap_or(DEFAULT_RETRY_INTERVAL_MS);

        let retry_max_interval_ms = cli_args
            .retry_max_interval_ms
            .or(env_vars.retry_max_interval_ms)
            .unwrap_or(DEFAULT_RETRY_MAX_INTERVAL_MS);

        let retries: usize = cli_args
            .retries
            .or(env_vars.retries)
            .unwrap_or(DEFAULT_RETRIES);

        let progress_bar_style: ProgressStyle = ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )
            .unwrap();
        let spinner_style: ProgressStyle = ProgressStyle::default_spinner()
            .template("{spinner:.green} [{elapsed}] {msg}")
            .unwrap();

        Config {
            pg_host,
            pg_user,
            pg_pass,
            api_concurrency_limit,
            max_db_connections,
            db_concurrency_limit,
            db_query_batch_size,
            db_query_batch_timeout_ms,
            reset_db,
            retry_interval_ms,
            retry_max_interval_ms,
            retries,
            progress_bar_style,
            spinner_style,
        }
    }
}
