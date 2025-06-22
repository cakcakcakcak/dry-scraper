//! Loads and manages application environment variables, providing defaults for optional values.
//!
//! This module uses the `dotenvy` crate to load environment variables from a `.env` file if present,
//! and provides a static `ENVIRONMENT_VARIABLES` instance for global access. Required environment
//! variables are:
//! - `PG_HOST`
//! - `PG_USER`
//! - `PG_PASS`
//!
//! Optional environment variables with defaults:
//! - `MAX_DB_CONNECTIONS` (default: 5)
//! - `DEV_MODE` (default: false)
//! - `RETRY_JITTER_DURATION_MS` (default: 100)
//! - `RETRIES` (default: 5)
//!
//! If a required variable is missing or invalid, the application will panic with an error message.
//! Optional variables fall back to their defaults if missing or invalid, with a debug log message.
//!
//! # Example
//! ```rust
//! let env = &ENVIRONMENT_VARIABLES;
//! println!("Postgres host: {}", env.pg_host);
//! ```
//!
//! # Errors
//! Returns an `LPError::Env` if a required environment variable is missing or not unicode.
//!
//! # Testing
//! Includes tests for missing and invalid environment variables, ensuring correct error handling
//! and default value usage.

use dotenvy;

use once_cell::sync::Lazy;
use tracing::instrument;

use crate::LPError;

const DEFAULT_MAX_DB_CONNECTIONS: u32 = 5;
const DEFAULT_RETRY_JITTER_DURATION_MS: u64 = 100;
const DEFAULT_RETRIES: usize = 5;

pub static ENVIRONMENT_VARIABLES: Lazy<EnvironmentVariables> = Lazy::new(|| {
    let span = tracing::span!(tracing::Level::INFO, "env_init");
    let _enter = span.enter();

    match dotenvy::dotenv() {
        Ok(_) => tracing::debug!("Loaded .env file"),
        Err(dotenvy::Error::Io(_)) => tracing::debug!(".env file not found, skipping"),
        Err(e) => tracing::warn!("Error loading .env file: {e}"),
    }
    EnvironmentVariables::from_env().unwrap_or_else(|e| {
        tracing::error!(
            error = %e,
            "Failed to load required environment variables"
        );
        panic!(
            r#"Failed to load required environment variables: {e}\n
            At minimum, environment variables `PG_HOST`, `PG_USER`, and `PG_PASS`
            must be defined at runtime or in a file named `.env`.\nPanicking!"#)
    })
});

#[derive(Debug)]
pub struct EnvironmentVariables {
    pub pg_host: String,
    pub pg_user: String,
    pub pg_pass: String,
    pub max_db_connections: u32,
    pub dev_mode: bool,
    pub retry_jitter_duration_ms: u64,
    pub retries: usize,
}
impl EnvironmentVariables {
    #[instrument]
    pub fn from_env() -> Result<Self, LPError> {
        Ok(Self {
            pg_host: Self::get_environment_variable("PG_HOST")?,
            pg_user: Self::get_environment_variable("PG_USER")?,
            pg_pass: Self::get_environment_variable("PG_PASS")?,

            max_db_connections: Self::get_environment_variable("MAX_DB_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| { 
                    tracing::debug!(
                        "Environment variable missing or invalid. Instead using default MAX_DB_CONNECTIONS = {}.",
                        DEFAULT_MAX_DB_CONNECTIONS
                    );
                    DEFAULT_MAX_DB_CONNECTIONS}),

            dev_mode: Self::get_environment_variable("DEV_MODE")
                .unwrap_or_else(|_| "false".to_string()) == "true",
                
            retry_jitter_duration_ms: Self::get_environment_variable("RETRY_JITTER_DURATION_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| {
                    tracing::debug!(
                        "Environment variable missing or invalid. Instead using default RETRY_JITTER_DURATION_MS = {}.",
                        DEFAULT_RETRY_JITTER_DURATION_MS
                    );
                    DEFAULT_RETRY_JITTER_DURATION_MS}),
                    
            retries: Self::get_environment_variable("RETRIES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| {
                    tracing::debug!(
                        "Environment variable missing or invalid. Instead using default RETRIES = {}.",
                        DEFAULT_RETRIES
                    );
                    DEFAULT_RETRIES
                }),
        })
    }

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
}

#[cfg(test)]
#[serial_test::serial]
mod tests {
    use super::*;

    #[test]
    fn test_env_var_missing_pg_host() {
        unsafe {
            std::env::remove_var("PG_HOST");
            std::env::set_var("PG_USER", "pg_user");
            std::env::set_var("PG_PASS", "pg_pass");
            std::env::set_var("MAX_DB_CONNECTIONS", "5");
            std::env::set_var("DEV_MODE", "true");
            std::env::set_var("RETRY_JITTER_DURATION_MS", "100");
            std::env::set_var("RETRIES", "5");
        }

        let err = EnvironmentVariables::from_env().unwrap_err();
        assert!(
            matches!(err, LPError::Env(msg) if msg.contains("Environment variable PG_HOST not set."))
        );
    }

    #[test]
    fn test_env_var_missing_pg_user() {
        unsafe {
            std::env::set_var("PG_HOST", "pg_host");
            std::env::remove_var("PG_USER");
            std::env::set_var("PG_PASS", "pg_pass");
            std::env::set_var("MAX_DB_CONNECTIONS", "5");
            std::env::set_var("DEV_MODE", "true");
            std::env::set_var("RETRY_JITTER_DURATION_MS", "100");
            std::env::set_var("RETRIES", "5");
        }

        let err = EnvironmentVariables::from_env().unwrap_err();
        assert!(
            matches!(err, LPError::Env(msg) if msg.contains("Environment variable PG_USER not set."))
        );
    }

    #[test]
    fn test_env_var_missing_pg_pass() {
        unsafe {
            std::env::set_var("PG_HOST", "pg_host");
            std::env::set_var("PG_USER", "pg_user");
            std::env::remove_var("PG_PASS");
            std::env::set_var("MAX_DB_CONNECTIONS", "5");
            std::env::set_var("DEV_MODE", "true");
            std::env::set_var("RETRY_JITTER_DURATION_MS", "100");
            std::env::set_var("RETRIES", "5");
        }

        let err = EnvironmentVariables::from_env().unwrap_err();
        assert!(
            matches!(err, LPError::Env(msg) if msg.contains("Environment variable PG_PASS not set."))
        );
    }

    #[test]
    fn test_env_var_missing_max_db_connections() {
        unsafe {
            std::env::set_var("PG_HOST", "pg_host");
            std::env::set_var("PG_USER", "pg_user");
            std::env::set_var("PG_PASS", "pg_pass");
            std::env::remove_var("MAX_DB_CONNECTIONS");
            std::env::set_var("DEV_MODE", "true");
            std::env::set_var("RETRY_JITTER_DURATION_MS", "100");
            std::env::set_var("RETRIES", "5");
        }

        let env = EnvironmentVariables::from_env().unwrap();
        assert!(env.max_db_connections == DEFAULT_MAX_DB_CONNECTIONS);
    }

    #[test]
    fn test_env_var_invalid_max_db_connections() {
        unsafe {
            std::env::set_var("PG_HOST", "pg_host");
            std::env::set_var("PG_USER", "pg_user");
            std::env::set_var("PG_PASS", "pg_pass");
            std::env::set_var("MAX_DB_CONNECTIONS", "notanumber");
            std::env::set_var("DEV_MODE", "true");
            std::env::set_var("RETRY_JITTER_DURATION_MS", "100");
            std::env::set_var("RETRIES", "5");
        }

        let env = EnvironmentVariables::from_env().unwrap();
        assert!(env.max_db_connections == DEFAULT_MAX_DB_CONNECTIONS);
    }

    #[test]
    fn test_env_var_missing_dev_mode() {
        unsafe {
            std::env::set_var("PG_HOST", "pg_host");
            std::env::set_var("PG_USER", "pg_user");
            std::env::set_var("PG_PASS", "pg_pass");
            std::env::set_var("MAX_DB_CONNECTIONS", "5");
            std::env::remove_var("DEV_MODE");
            std::env::set_var("RETRY_JITTER_DURATION_MS", "100");
            std::env::set_var("RETRIES", "5");
        }

        let env = EnvironmentVariables::from_env().unwrap();
        assert!(!env.dev_mode);
    }

    #[test]
    fn test_env_var_invalid_dev_node() {
        unsafe {
            std::env::set_var("PG_HOST", "pg_host");
            std::env::set_var("PG_USER", "pg_user");
            std::env::set_var("PG_PASS", "pg_pass");
            std::env::set_var("MAX_DB_CONNECTIONS", "5");
            std::env::set_var("DEV_MODE", "nottrueorfalse");
            std::env::set_var("RETRY_JITTER_DURATION_MS", "100");
            std::env::set_var("RETRIES", "5");
        }

        let env = EnvironmentVariables::from_env().unwrap();
        assert!(!env.dev_mode);
    }

    #[test]
    fn test_env_var_missing_retry_jitter_duration_ms() {
        unsafe {
            std::env::set_var("PG_HOST", "pg_host");
            std::env::set_var("PG_USER", "pg_user");
            std::env::set_var("PG_PASS", "pg_pass");
            std::env::set_var("MAX_DB_CONNECTIONS", "5");
            std::env::set_var("DEV_MODE", "true");
            std::env::remove_var("RETRY_JITTER_DURATION_MS");
            std::env::set_var("RETRIES", "5");
        }

        let env = EnvironmentVariables::from_env().unwrap();
        assert!(env.retry_jitter_duration_ms == DEFAULT_RETRY_JITTER_DURATION_MS);
    }

    #[test]
    fn test_env_var_invalid_retry_jitter_duration_ms() {
        unsafe {
            std::env::set_var("PG_HOST", "pg_host");
            std::env::set_var("PG_USER", "pg_user");
            std::env::set_var("PG_PASS", "pg_pass");
            std::env::set_var("MAX_DB_CONNECTIONS", "5");
            std::env::set_var("DEV_MODE", "true");
            std::env::set_var("RETRY_JITTER_DURATION_MS", "100");
            std::env::set_var("RETRIES", "5");
        }

        let env = EnvironmentVariables::from_env().unwrap();
        assert!(env.retry_jitter_duration_ms == DEFAULT_RETRY_JITTER_DURATION_MS);
    }

    #[test]
    fn test_env_var_missing_retries() {
        unsafe {
            std::env::set_var("PG_HOST", "pg_host");
            std::env::set_var("PG_USER", "pg_user");
            std::env::set_var("PG_PASS", "pg_pass");
            std::env::set_var("MAX_DB_CONNECTIONS", "5");
            std::env::set_var("DEV_MODE", "true");
            std::env::set_var("RETRY_JITTER_DURATION_MS", "100");
            std::env::remove_var("RETRIES");
        }

        let env = EnvironmentVariables::from_env().unwrap();
        assert!(env.retries == DEFAULT_RETRIES);
    }

    #[test]
    fn test_env_var_invalid_retries() {
        unsafe {
            std::env::set_var("PG_HOST", "pg_host");
            std::env::set_var("PG_USER", "pg_user");
            std::env::set_var("PG_PASS", "pg_pass");
            std::env::set_var("MAX_DB_CONNECTIONS", "5");
            std::env::set_var("DEV_MODE", "true");
            std::env::set_var("RETRY_JITTER_DURATION_MS", "100");
            std::env::set_var("RETRIES", "notanumber");
        }

        let env = EnvironmentVariables::from_env().unwrap();
        assert!(env.retries == DEFAULT_RETRIES);
    }
}
