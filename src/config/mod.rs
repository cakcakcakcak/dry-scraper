use serde::{Deserialize, Serialize};

use clap::Parser;
use indicatif::ProgressStyle;
use once_cell::sync::Lazy;

pub mod cli_args;
pub mod env_vars;

use cli_args::CliArgs;
use env_vars::EnvironmentVariables;

const DEFAULT_API_CONCURRENCY_LIMIT: usize = 32;
const DEFAULT_MAX_DB_CONNECTIONS: u32 = 32;
const DEFAULT_DB_CONCURRENCY_LIMIT: usize = 16;
const DEFAULT_DB_QUERY_BATCH_SIZE: usize = 1_000;
const DEFAULT_DB_QUERY_BATCH_TIMEOUT_MS: u64 = 2_000;
const DEFAULT_RETRY_INTERVAL_MS: u64 = 100;
const DEFAULT_RETRY_MAX_INTERVAL_MS: u64 = 10_000;
const DEFAULT_RETRIES: usize = 5;
const DEFAULT_PROGRESS_BAR_STYLE_FORMAT: &str =
    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) - {msg}";
const DEFAULT_PROGRESS_SPINNER_STYLE_FORMAT: &str = "{spinner:.green} [{elapsed}] {msg}";

pub static DEFAULT_PROGRESS_BAR_STYLE: Lazy<ProgressStyle> = Lazy::new(|| {
    indicatif::ProgressStyle::with_template(DEFAULT_PROGRESS_BAR_STYLE_FORMAT)
        .expect("DEFAULT_PROGRESS_BAR_STYLE_FORMAT must be a valid indicatif template")
});
pub static DEFAULT_PROGRESS_SPINNER_STYLE: Lazy<ProgressStyle> = Lazy::new(|| {
    indicatif::ProgressStyle::with_template(DEFAULT_PROGRESS_SPINNER_STYLE_FORMAT)
        .expect("DEFAULT_PROGRESS_SPINNER_STYLE_FORMAT must be a valid indicatif template")
});

pub static CONFIG: Lazy<Config> = Lazy::new(Config::from_env_and_args);
pub static UI_CONFIG: Lazy<UiTheme> = Lazy::new(|| UiTheme::from_config(&CONFIG));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub api_concurrency_limit: usize,
    pub max_db_connections: u32,
    pub db_concurrency_limit: usize,
    pub db_query_batch_size: usize,
    pub db_query_batch_timeout_ms: u64,
    pub retry_interval_ms: u64,
    pub retry_max_interval_ms: u64,
    pub retries: usize,
    pub progress_bar_style_format: String,
    pub progress_spinner_style_format: String,
}

pub struct UiTheme {
    pub progress_bar_style: ProgressStyle,
    pub progress_spinner_style: ProgressStyle,
}

impl Config {
    #[tracing::instrument]
    pub fn from_env_and_args() -> Self {
        let cli_args: CliArgs = CliArgs::parse();
        let env_vars: EnvironmentVariables = EnvironmentVariables::from_env();

        let database_url: String = cli_args.database_url.or(env_vars.database_url).unwrap_or_else(|| {
            panic!(
                r#"Failed to load required environment variable: `DATABASE_URL`
                At minimum, `DATABASE_URL` must be defined 1) as environment variables, 2) in a file named `.env`, or 3) as command line arguments.
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

        let progress_bar_style_format: String = cli_args
            .progress_bar_style_format
            .or(env_vars.progress_bar_style_format)
            .unwrap_or(DEFAULT_PROGRESS_BAR_STYLE_FORMAT.to_string());

        let progress_spinner_style_format: String = cli_args
            .progress_spinner_style_format
            .or(env_vars.progress_spinner_style_format)
            .unwrap_or(DEFAULT_PROGRESS_SPINNER_STYLE_FORMAT.to_string());

        Config {
            database_url,
            api_concurrency_limit,
            max_db_connections,
            db_concurrency_limit,
            db_query_batch_size,
            db_query_batch_timeout_ms,
            retry_interval_ms,
            retry_max_interval_ms,
            retries,
            progress_bar_style_format,
            progress_spinner_style_format,
        }
    }
}

impl UiTheme {
    pub fn from_config(cfg: &Config) -> Self {
        let progress_bar_style = match ProgressStyle::with_template(&cfg.progress_bar_style_format)
        {
            Ok(style) => style,
            Err(e) => {
                tracing::warn!(
                    provided = %cfg.progress_bar_style_format,
                    error = %e,
                    "Invalid progress bar style format; falling back to default"
                );
                DEFAULT_PROGRESS_BAR_STYLE.clone()
            }
        };

        let progress_spinner_style =
            match ProgressStyle::with_template(&cfg.progress_spinner_style_format) {
                Ok(style) => style,
                Err(e) => {
                    tracing::warn!(
                        provided = %cfg.progress_spinner_style_format,
                        error = %e,
                        "Invalid progress spinner style format; falling back to default"
                    );
                    DEFAULT_PROGRESS_SPINNER_STYLE.clone()
                }
            };

        UiTheme {
            progress_bar_style,
            progress_spinner_style,
        }
    }
}

#[test]
fn default_progress_templates_parse() {
    assert!(ProgressStyle::with_template(DEFAULT_PROGRESS_BAR_STYLE_FORMAT).is_ok());
    assert!(ProgressStyle::with_template(DEFAULT_PROGRESS_SPINNER_STYLE_FORMAT).is_ok());
}
