use std::env::VarError;

#[derive(Debug)]
pub struct EnvironmentVariables {
    pub pg_host: Option<String>,
    pub pg_user: Option<String>,
    pub pg_pass: Option<String>,
    pub api_concurrency_limit: Option<usize>,
    pub max_db_connections: Option<u32>,
    pub db_concurrency_limit: Option<usize>,
    pub db_query_batch_size: Option<usize>,
    pub db_query_batch_timeout_ms: Option<u64>,
    pub reset_db: Option<bool>,
    pub retry_interval_ms: Option<u64>,
    pub retry_max_interval_ms: Option<u64>,
    pub retries: Option<usize>,
    pub progress_bar_style_format: Option<String>,
    pub progress_spinner_style_format: Option<String>,
}
impl EnvironmentVariables {
    pub fn from_env() -> Self {
        Self {
            pg_host: Self::get_parsed_env_var_with_log("PG_HOST"),
            pg_user: Self::get_parsed_env_var_with_log("PG_USER"),
            pg_pass: Self::get_parsed_env_var_with_log("PG_PASS"),
            api_concurrency_limit: Self::get_parsed_env_var_with_log("API_CONCURRENCY_LIMIT"),
            max_db_connections: Self::get_parsed_env_var_with_log("MAX_DB_CONNECTIONS"),
            db_concurrency_limit: Self::get_parsed_env_var_with_log("DB_CONCURRENCY_LIMIT"),
            db_query_batch_size: Self::get_parsed_env_var_with_log("DB_QUERY_BATCH_SIZE"),
            db_query_batch_timeout_ms: Self::get_parsed_env_var_with_log(
                "DB_QUERY_BATCH_TIMEOUT_MS",
            ),
            reset_db: Self::get_parsed_env_var_with_log("RESET_DB"),
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
