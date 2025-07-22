use tracing::instrument;

use crate::LPError;

#[derive(Debug)]
pub struct EnvironmentVariables {
    pub pg_host: Option<String>,
    pub pg_user: Option<String>,
    pub pg_pass: Option<String>,
    pub season_limit: Option<usize>,
    pub max_db_connections: Option<u32>,
    pub upsert_concurrency: Option<usize>,
    pub reset_db: Option<bool>,
    pub retry_jitter_duration_ms: Option<u64>,
    pub retries: Option<usize>,
}
impl EnvironmentVariables {
    pub fn from_env() -> Self {
        Self {
            pg_host: Self::get_parsed_env_var_with_log("PG_HOST"),
            pg_user: Self::get_parsed_env_var_with_log("PG_USER"),
            pg_pass: Self::get_parsed_env_var_with_log("PG_PASS"),
            season_limit: Self::get_parsed_env_var_with_log("SEASON_LIMIT"),
            max_db_connections: Self::get_parsed_env_var_with_log("MAX_DB_CONNECTIONS"),
            upsert_concurrency: Self::get_parsed_env_var_with_log("UPSERT_CONCURRENCY"),
            reset_db: Self::get_parsed_env_var_with_log("RESET_DB"),
            retry_jitter_duration_ms: Self::get_parsed_env_var_with_log("RETRY_JITTER_DURATION_MS"),
            retries: Self::get_parsed_env_var_with_log("RETRIES"),
        }
    }

    #[tracing::instrument]
    fn get_environment_variable(key: &str) -> Result<String, LPError> {
        match std::env::var(key) {
            Ok(var) => Ok(var),
            Err(std::env::VarError::NotPresent) => Err(LPError::Env(format!(
                "Environment variable {} not set.",
                key
            ))),
            Err(std::env::VarError::NotUnicode(val)) => Err(LPError::Env(format!(
                "Environment variable {} not unicode.\nInstead found \"{:?}\"",
                key, val
            ))),
        }
    }

    #[tracing::instrument]
    fn get_parsed_env_var_with_log<T: std::str::FromStr>(key: &str) -> Option<T> {
        match Self::get_environment_variable(key) {
            Ok(val) => match val.parse::<T>() {
                Ok(parsed) => Some(parsed),
                Err(_) => {
                    tracing::debug!(
                        "Environment variable `{key}` present but could not be parsed to `{}`.",
                        std::any::type_name::<T>()
                    );
                    None
                }
            },
            Err(e) => {
                tracing::debug!("{e}");
                None
            }
        }
    }
}
