//! Initializes the PostgreSQL database and returns a connection pool.
//!
//! This function performs the following steps:
//! 1. Constructs the database URL from environment variables.
//! 2. Checks if the database exists, and creates it if it does not (with retry logic).
//! 3. Establishes a connection pool with a configurable maximum number of connections (with retry logic).
//! 4. Ensures the `public` schema exists in the database.
//! 5. If `DEV_MODE` is enabled, drops specific tables for a clean development state.
//! 6. Runs database migrations to ensure the schema is up to date.
//!
//! # Returns
//! * `Ok(sqlx::Pool<sqlx::Postgres>)` - The established connection pool.
//! * `Err(LPError)` - If any step fails.
//!
//! # Errors
//! Returns an `LPError` if:
//! - The database URL cannot be constructed.
//! - Database existence check or creation fails.
//! - Connection pool cannot be established.
//! - Schema creation or table dropping fails.
//! - Migrations fail.
//!
//! # Instrumentation
//! This function is instrumented for tracing and logs key events and errors.
//!
//! # Example
//! ```ignore
//! let pool = init_db().await?;
//! ```

use sqlx::migrate::MigrateDatabase;
use sqlx::postgres::PgPoolOptions;

use tracing::instrument;

use tokio_retry::RetryIf;

use crate::LPError;
use crate::config::env::ENVIRONMENT_VARIABLES;
use crate::sqlx_operation_with_retries;
use crate::util::{default_retry_strategy, is_transient_sqlx_error};

#[instrument]
pub async fn init_db() -> Result<sqlx::Pool<sqlx::Postgres>, LPError> {
    // get the environment variables and construct the database_url
    let db_url = database_url()?;
    tracing::debug!(db_url = %db_url, "Constructed lp database URL");

    // check if lp database exists, with retry strategy
    let db_exists =
        sqlx_operation_with_retries!(sqlx::Postgres::database_exists(&db_url).await).await?;
    tracing::info!(db_exists, "Checked if lp database exists");

    // if lp database does not exist, with retry strategy
    if !db_exists {
        tracing::warn!("lp database does not exist, attempting to create it.");
        _ = sqlx_operation_with_retries!(sqlx::Postgres::create_database(&db_url).await).await?;
        tracing::info!("lp database created");
    }

    // create a connection pool of `max_db_connections`, with retry strategy
    tracing::info!(
        max_db_connections = ENVIRONMENT_VARIABLES.max_db_connections,
        "Creating connection pool"
    );
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
    tracing::info!("Connection pool created.");

    // create public schema if it doesn't already exist
    tracing::debug!("Ensuring public schema exists");
    sqlx_operation_with_retries!(
        sqlx::query("CREATE SCHEMA IF NOT EXISTS public")
            .execute(&pool)
            .await
    )
    .await?;

    if ENVIRONMENT_VARIABLES.dev_mode {
        tracing::warn!("DEV_MODE enabled: dropping tables for clean state");
        sqlx_operation_with_retries!(
            sqlx::query("DROP TABLE IF EXISTS _sqlx_migrations")
                .execute(&pool)
                .await
        )
        .await?;
        sqlx_operation_with_retries!(
            sqlx::query("DROP TABLE IF EXISTS nhl_team")
                .execute(&pool)
                .await
        )
        .await?;
        sqlx_operation_with_retries!(
            sqlx::query("DROP TABLE IF EXISTS nhl_franchise")
                .execute(&pool)
                .await
        )
        .await?;
        sqlx_operation_with_retries!(
            sqlx::query("DROP TABLE IF EXISTS nhl_season")
                .execute(&pool)
                .await
        )
        .await?;
        sqlx_operation_with_retries!(
            sqlx::query("DROP TABLE IF EXISTS api_cache")
                .execute(&pool)
                .await
        )
        .await?;
        tracing::info!("Dropped tables in DEV_MODE");
    }

    // run migrations
    tracing::info!("Running database migrations...");
    sqlx::migrate!().run(&pool).await?;
    tracing::info!("Database migrations complete");

    Ok(pool)
}

fn database_url() -> Result<String, LPError> {
    let pg_host = &ENVIRONMENT_VARIABLES.pg_host;
    let pg_user = &*ENVIRONMENT_VARIABLES.pg_user;
    let pg_pass = &*ENVIRONMENT_VARIABLES.pg_pass;

    Ok(format!("postgres://{pg_user}:{pg_pass}@{pg_host}/lp"))
}
