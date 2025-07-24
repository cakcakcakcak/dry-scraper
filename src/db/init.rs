use sqlx::migrate::MigrateDatabase;
use sqlx::postgres::PgPoolOptions;

use tokio_retry::RetryIf;

use crate::LPError;
use crate::config::CONFIG;
use crate::db::DbPool;
use crate::sqlx_operation_with_retries;
use crate::util::{default_retry_strategy, is_transient_sqlx_error};

#[tracing::instrument]
pub async fn init_db() -> Result<DbPool, LPError> {
    let db_url: String = database_url()?;
    tracing::debug!(db_url);

    let db_exists =
        sqlx_operation_with_retries!(sqlx::Postgres::database_exists(&db_url).await).await?;
    tracing::debug!(db_exists, "Checked if lp database exists");

    if !db_exists {
        tracing::warn!("lp database does not exist, attempting to create it.");
        _ = sqlx_operation_with_retries!(sqlx::Postgres::create_database(&db_url).await).await?;
        tracing::info!("lp database created");
    }

    tracing::info!(
        max_db_connections = CONFIG.max_db_connections,
        "Creating connection pool"
    );
    let pool: sqlx::Pool<sqlx::Postgres> = RetryIf::spawn(
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
            sqlx::query("DROP TABLE IF EXISTS nhl_play")
                .execute(&pool)
                .await
        )
        .await?;
        sqlx_operation_with_retries!(
            sqlx::query("DROP TABLE IF EXISTS nhl_roster_spot")
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
    let pg_host: &str = &*CONFIG.pg_host;
    let pg_user: &str = &*CONFIG.pg_user;
    let pg_pass: &str = &*CONFIG.pg_pass;

    Ok(format!("postgres://{pg_user}:{pg_pass}@{pg_host}/lp"))
}
