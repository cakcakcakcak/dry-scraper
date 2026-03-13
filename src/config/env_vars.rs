use std::env::VarError;

#[derive(Debug)]
pub struct EnvironmentVariables {
    pub database_url: Option<String>,
    pub api_concurrency_limit: Option<usize>,
    pub api_delay_ms: Option<u64>,
    pub nhl_api_rate_limit: Option<u32>,
    pub max_db_connections: Option<u32>,
    pub db_concurrency_limit: Option<usize>,
    pub db_query_batch_size: Option<usize>,
    pub db_query_batch_timeout_ms: Option<u64>,

    pub retry_interval_ms: Option<u64>,
    pub retry_max_interval_ms: Option<u64>,
    pub retries: Option<usize>,
    pub progress_bar_style_format: Option<String>,
    pub progress_spinner_style_format: Option<String>,
}
impl EnvironmentVariables {
    pub fn from_env() -> Self {
        Self {
            database_url: Self::get_parsed_env_var_with_log("DATABASE_URL"),
            api_concurrency_limit: Self::get_parsed_env_var_with_log("API_CONCURRENCY_LIMIT"),
            api_delay_ms: Self::get_parsed_env_var_with_log("API_DELAY_MS"),
            nhl_api_rate_limit: Self::get_parsed_env_var_with_log("NHL_API_RATE_LIMIT"),
            max_db_connections: Self::get_parsed_env_var_with_log("MAX_DB_CONNECTIONS"),
            db_concurrency_limit: Self::get_parsed_env_var_with_log("DB_CONCURRENCY_LIMIT"),
            db_query_batch_size: Self::get_parsed_env_var_with_log("DB_QUERY_BATCH_SIZE"),
            db_query_batch_timeout_ms: Self::get_parsed_env_var_with_log(
                "DB_QUERY_BATCH_TIMEOUT_MS",
            ),

            retry_interval_ms: Self::get_parsed_env_var_with_log("RETRY_INTERVAL_MS"),
            retry_max_interval_ms: Self::get_parsed_env_var_with_log("RETRY_MAX_INTERVAL_MS"),
            retries: Self::get_parsed_env_var_with_log("RETRIES"),
            progress_bar_style_format: Self::get_parsed_env_var_with_log(
                "PROGRESS_BAR_STYLE_FORMAT",
            ),
            progress_spinner_style_format: Self::get_parsed_env_var_with_log(
                "PROGRESS_SPINNER_STYLE_FORMAT",
            ),
        }
    }

    #[tracing::instrument]
    fn get_environment_variable(key: &str) -> Result<String, VarError> {
        match std::env::var(key) {
            Ok(var) => Ok(var),
            Err(e) => Err(e),
        }
    }

    #[tracing::instrument]
    fn get_parsed_env_var_with_log<T: std::str::FromStr>(key: &str) -> Option<T> {
        match Self::get_environment_variable(key) {
            Ok(val) => match val.parse::<T>() {
                Ok(parsed) => Some(parsed),
                Err(_) => {
                    tracing::warn!(
                        "Environment variable `{key}` present but could not be parsed to `{}`.",
                        std::any::type_name::<T>()
                    );
                    None
                }
            },
            Err(std::env::VarError::NotPresent) => {
                tracing::debug!(
                    "Environment variable {key} not set: `std::env::VarError::NotPresent`"
                );
                None
            }
            Err(std::env::VarError::NotUnicode(val)) => {
                tracing::warn!(
                    "Environment variable {key} not unicode. Instead found \"{val:?}\".: `std::env::VarError::NotUnicode`"
                );
                None
            }
        }
    }
}
