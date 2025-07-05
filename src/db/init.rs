use sqlx::migrate::MigrateDatabase;
use sqlx::postgres::PgPoolOptions;

use tracing::instrument;

use tokio_retry::RetryIf;

use crate::LPError;
use crate::config::CONFIG;
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
        max_db_connections = CONFIG.max_db_connections,
        "Creating connection pool"
    );
    let pool = RetryIf::spawn(
        default_retry_strategy(),
        || async {
            PgPoolOptions::new()
                .max_connections(CONFIG.max_db_connections)
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

    if CONFIG.reset_db {
        tracing::warn!("RESET_DB enabled: dropping tables and enums for clean state");
        sqlx_operation_with_retries!(
            sqlx::query("DROP TABLE IF EXISTS nhl_playoff_series")
                .execute(&pool)
                .await
        )
        .await?;
        sqlx_operation_with_retries!(
            sqlx::query("DROP TABLE IF EXISTS nhl_game")
                .execute(&pool)
                .await
        )
        .await?;
        sqlx_operation_with_retries!(
            sqlx::query("DROP TYPE IF EXISTS game_type")
                .execute(&pool)
                .await
        )
        .await?;
        sqlx_operation_with_retries!(
            sqlx::query("DROP TYPE IF EXISTS period_type")
                .execute(&pool)
                .await
        )
        .await?;
        sqlx_operation_with_retries!(
            sqlx::query("DROP TABLE IF EXISTS nhl_player")
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
            sqlx::query("DROP TABLE IF EXISTS _sqlx_migrations")
                .execute(&pool)
                .await
        )
        .await?;
        tracing::info!("Dropped tables in RESET_DB");
    }

    // run migrations
    tracing::info!("Running database migrations...");
    sqlx::migrate!().run(&pool).await?;
    tracing::info!("Database migrations complete");

    Ok(pool)
}

fn database_url() -> Result<String, LPError> {
    let pg_host = &*CONFIG.pg_host;
    let pg_user = &*CONFIG.pg_user;
    let pg_pass = &*CONFIG.pg_pass;

    Ok(format!("postgres://{pg_user}:{pg_pass}@{pg_host}/lp"))
}
