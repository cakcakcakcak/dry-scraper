use std::sync::Arc;

use dashmap::DashSet;
use sqlx::{migrate::MigrateDatabase, postgres::PgPoolOptions};
use tokio_retry::RetryIf;

use crate::{
    common::{
        db::{start_sqlx_worker, worker::WorkerConfig, CacheKey, DbPool, SqlxJobSender},
        errors::DSError,
        util::{default_retry_strategy, is_transient_sqlx_error},
    },
    config::Config,
    sqlx_operation_with_retries,
};

#[derive(Clone)]
pub struct DbContext {
    pub pool: DbPool,
    pub sqlx_tx: SqlxJobSender,
    pub key_cache: Arc<DashSet<CacheKey>>,
    pub config: Arc<Config>,
}
impl DbContext {
    pub async fn connect(cfg: &Config) -> Result<DbContext, DSError> {
        let pool = init_db(cfg).await?;
        let sqlx_tx = start_sqlx_worker(
            pool.clone(),
            WorkerConfig {
                batch_size: cfg.db_query_batch_size,
                batch_timeout_ms: cfg.db_query_batch_timeout_ms,
            },
        );
        Ok(DbContext {
            pool,
            sqlx_tx,
            key_cache: Arc::new(DashSet::new()),
            config: Arc::new(cfg.clone()),
        })
    }
}

#[tracing::instrument(skip(cfg))]
pub async fn init_db(cfg: &Config) -> Result<DbPool, DSError> {
    let db_url: String = cfg.database_url.clone();
    tracing::debug!(db_url);

    let db_exists =
        sqlx_operation_with_retries!(cfg, sqlx::Postgres::database_exists(&db_url).await).await?;
    tracing::debug!(db_exists, "Checked if lp database exists");

    if !db_exists {
        tracing::warn!("lp database does not exist, attempting to create it.");
        sqlx_operation_with_retries!(cfg, sqlx::Postgres::create_database(&db_url).await).await?;
        tracing::info!("lp database created");
    }

    tracing::info!(
        max_db_connections = cfg.max_db_connections,
        "Creating connection pool"
    );
    let pool: sqlx::Pool<sqlx::Postgres> = RetryIf::spawn(
        default_retry_strategy(cfg),
        || async {
            PgPoolOptions::new()
                .max_connections(cfg.max_db_connections)
                .connect(&db_url)
                .await
        },
        is_transient_sqlx_error,
    )
    .await?;
    tracing::info!("Connection pool created.");

    tracing::debug!("Ensuring public schema exists");
    sqlx_operation_with_retries!(
        cfg,
        sqlx::query("CREATE SCHEMA IF NOT EXISTS public")
            .execute(&pool)
            .await
    )
    .await?;

    // run migrations
    tracing::info!("Running database migrations...");
    sqlx::migrate!().run(&pool).await?;
    tracing::info!("Database migrations complete");

    Ok(pool)
}

#[cfg(debug_assertions)]
pub async fn reset_schema(pool: &DbPool, cfg: &Config) -> Result<(), DSError> {
    tracing::warn!("Resetting database schema (debug build only)");
    tracing::info!(
        "Dropping all tables except infrastructure tables (api_cache, data_source_error)"
    );

    // Infrastructure tables to preserve
    let preserve_tables = ["api_cache", "data_source_error", "_sqlx_migrations"];

    // Get all tables in public schema
    let tables: Vec<(String,)> = sqlx_operation_with_retries!(
        cfg,
        sqlx::query_as::<_, (String,)>(
            "SELECT tablename FROM pg_tables WHERE schemaname = 'public'"
        )
        .fetch_all(pool)
        .await
    )
    .await?;

    // Drop all tables except infrastructure
    for (table,) in tables {
        if !preserve_tables.contains(&table.as_str()) {
            tracing::debug!("Dropping table: {}", table);
            sqlx_operation_with_retries!(
                cfg,
                sqlx::query(&format!("DROP TABLE IF EXISTS \"{}\" CASCADE", table))
                    .execute(pool)
                    .await
            )
            .await?;
        }
    }

    // Drop custom types
    let types: Vec<(String,)> = sqlx_operation_with_retries!(
        cfg,
        sqlx::query_as::<_, (String,)>(
            "SELECT typname FROM pg_type WHERE typnamespace = (SELECT oid FROM pg_namespace WHERE nspname = 'public') AND typtype = 'e'"
        )
        .fetch_all(pool)
        .await
    )
    .await?;

    for (type_name,) in types {
        tracing::debug!("Dropping type: {}", type_name);
        sqlx_operation_with_retries!(
            cfg,
            sqlx::query(&format!("DROP TYPE IF EXISTS \"{}\" CASCADE", type_name))
                .execute(pool)
                .await
        )
        .await?;
    }

    // Re-run migrations to recreate dropped tables
    sqlx::migrate!("./migrations").run(pool).await?;

    tracing::info!("Database schema reset complete");
    Ok(())
}
