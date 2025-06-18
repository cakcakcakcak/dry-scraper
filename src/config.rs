use dotenvy;

use once_cell::sync::Lazy;

use sqlx::migrate::MigrateDatabase;
use sqlx::postgres::PgPoolOptions;

use tokio_retry::RetryIf;

use crate::LPError;
use crate::util::{default_retry_strategy, is_transient_sqlx_error};
use crate::sqlx_operation_with_retries;

const DEFAULT_MAX_DB_CONNECTIONS: u32 = 5;
const DEFAULT_RETRY_JITTER_DURATION_MS: u64 = 100;
const DEFAULT_RETRIES: usize = 5;


pub static ENVIRONMENT_VARIABLES: Lazy<EnvironmentVariables> = Lazy::new(|| {
    _ = dotenvy::dotenv();
    EnvironmentVariables::from_env().unwrap_or_else(|e| {
        panic!(
            r#"Failed to load required environment variables: {e}\n
            At minimum, environment variables `PG_HOST`, `PG_USER`, and `PG_PASS`
            must be defined at runtime or defined in a file named `.env`.\nPanicking!"#)
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
    pub fn from_env() -> Result<Self, LPError> {
        Ok(Self {
            pg_host: Self::get_environment_variable("PG_HOST")?,
            pg_user: Self::get_environment_variable("PG_USER")?,
            pg_pass: Self::get_environment_variable("PG_PASS")?,
            max_db_connections: Self::get_environment_variable("MAX_DB_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| { 
                    println!(
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
                    println!(
                        "Environment variable missing or invalid. Instead using default RETRY_JITTER_DURATION_MS = {}.",
                        DEFAULT_RETRY_JITTER_DURATION_MS
                    );
                    DEFAULT_RETRY_JITTER_DURATION_MS}),
            retries: Self::get_environment_variable("RETRIES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| {
                    println!(
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


pub async fn init_db() -> Result<sqlx::Pool<sqlx::Postgres>, LPError> {

    // get the environment variables and construct the database_url
    let db_url = database_url()?;

    // check if lp database exists, with retry strategy
    let db_exists = sqlx_operation_with_retries!(
        sqlx::Postgres::database_exists(&db_url).await
    )?;

    // if lp database does not exist, with retry strategy
    if !db_exists {
        _ = sqlx_operation_with_retries!(
            sqlx::Postgres::create_database(&db_url).await
        )?;
    }

    // create a connection pool of `max_db_connections`, with retry strategy
    let pool = RetryIf::spawn(
        default_retry_strategy(),
        || async {
            PgPoolOptions::new()
                .max_connections(ENVIRONMENT_VARIABLES.max_db_connections)
                .connect(&db_url)
                .await
        },
        is_transient_sqlx_error,
    )
    .await?;

    // create public schema if it doesn't already exist
    sqlx_operation_with_retries!(
        sqlx::query("CREATE SCHEMA IF NOT EXISTS public")
        .execute(&pool)
        .await
    )?;

    if ENVIRONMENT_VARIABLES.dev_mode {
        sqlx_operation_with_retries!(
            sqlx::query("DROP TABLE IF EXISTS _sqlx_migrations")
            .execute(&pool)
            .await
        )?;
        sqlx_operation_with_retries!(
            sqlx::query("DROP TABLE IF EXISTS nhl_season")
            .execute(&pool)
            .await
        )?;
        sqlx_operation_with_retries!(
            sqlx::query("DROP TABLE IF EXISTS api_cache")
            .execute(&pool)
            .await
        )?;
    }

    // run migrations
    sqlx::migrate!().run(&pool).await?;

    Ok(pool)
}

fn database_url() -> Result<String, LPError> {
    let pg_host = &ENVIRONMENT_VARIABLES.pg_host;
    let pg_user = &*ENVIRONMENT_VARIABLES.pg_user;
    let pg_pass = &*ENVIRONMENT_VARIABLES.pg_pass;

    Ok(format!("postgres://{pg_user}:{pg_pass}@{pg_host}/lp"))
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
            std::env::set_var("MAX_DB_CONNECTIONS", "notanumber");
            std::env::set_var("DEV_MODE", "true");
            std::env::set_var("RETRY_JITTER_DURATION_MS", "notanumber");
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
            std::env::set_var("MAX_DB_CONNECTIONS", "notanumber");
            std::env::set_var("DEV_MODE", "true");
            std::env::set_var("RETRY_JITTER_DURATION_MS", "5");
            std::env::set_var("RETRIES", "notanumber");
        }

        let env = EnvironmentVariables::from_env().unwrap();
        assert!(env.retries == DEFAULT_RETRIES);
    }

    #[tokio::test]
    async fn test_init_db_success() {
        // Call your function
        let result = init_db().await;

        // Assert success or check the pool
        assert!(result.is_ok());
    }
}
