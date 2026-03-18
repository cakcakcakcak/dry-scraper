#![allow(async_fn_in_trait)]

use futures::stream::{self, StreamExt};
use std::{cmp::Eq, fmt::Debug, hash::Hash};

use async_trait::async_trait;
use sqlx::{postgres::PgQueryResult, FromRow};

use crate::{
    common::{
        app_context::AppContext,
        db::{
            CacheKey, DbContext, SqlxJob, SqlxJobOrFlush, SqlxJobResult, StaticPgQuery,
            StaticPgQueryAs,
        },
        errors::DSError,
        models::{partition_and_track_errors, traits::HasTypeName},
    },
    sqlx_operation_with_retries,
};

#[async_trait]
pub trait DbEntity:
    Debug
    + Sized
    + Clone
    + Send
    + Sync
    + Unpin
    + HasTypeName
    + for<'a> FromRow<'a, sqlx::postgres::PgRow>
    + 'static
{
    type Pk: PrimaryKey<Entity = Self>;

    fn pk(&self) -> Self::Pk;

    fn cache_key(&self) -> CacheKey {
        self.pk().cache_key()
    }

    fn foreign_keys(&self) -> Vec<CacheKey> {
        vec![]
    }

    fn fmt_debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.pk(), f)
    }

    async fn warm_key_cache(db_context: &DbContext) -> Result<(), DSError> {
        let select_query: StaticPgQueryAs<Self::Pk> = Self::select_key_query();
        let key_vec: Vec<Self::Pk> = select_query.fetch_all(&db_context.pool).await?;
        for entity in key_vec {
            db_context.key_cache.insert(entity.cache_key());
        }
        Ok(())
    }

    fn upsert_query(&self) -> StaticPgQuery;

    fn select_key_query() -> StaticPgQueryAs<Self::Pk>;

    #[tracing::instrument(skip(db_context))]
    async fn verify_by_key(
        db_context: &DbContext,
        id: Self::Pk,
    ) -> Result<Option<Self::Pk>, DSError> {
        if db_context.key_cache.contains(&id.cache_key()) {
            return Ok(None);
        }
        match Self::fetch_from_db_by_key(db_context, &id).await {
            Ok(Some(_)) => Ok(None),
            Ok(None) => Ok(Some(id)),
            Err(e) => return Err(e),
        }
    }

    #[tracing::instrument(skip(db_context))]
    async fn fetch_from_db_by_key(
        db_context: &DbContext,
        id: &Self::Pk,
    ) -> Result<Option<Self>, DSError> {
        match sqlx_operation_with_retries!(
            &db_context.config,
            id.create_select_query()
                .fetch_optional(&db_context.pool)
                .await
        )
        .await
        {
            Ok(Some(row)) => {
                db_context.key_cache.insert(id.cache_key());
                Self::from_row(&row).map(Some).map_err(DSError::from)
            }
            Ok(None) => Ok(None),
            Err(e) => {
                tracing::error!(key = ?id, error = %e, "Failed to fetch from database");
                Err(DSError::from(e))
            }
        }
    }

    #[tracing::instrument(skip(self, db_context))]
    async fn upsert(&self, db_context: &DbContext) -> Result<PgQueryResult, DSError> {
        let (result_tx, result_rx) = tokio::sync::oneshot::channel::<SqlxJobResult>();

        let query = self.upsert_query();
        let job = SqlxJob { query, result_tx };
        if let Err(e) = db_context.sqlx_tx.send(SqlxJobOrFlush::Job(job)).await {
            tracing::warn!("Failed to send job: {e:?}");
        }

        match result_rx.await {
            Ok(Ok(pg_result)) => {
                db_context.key_cache.insert(self.cache_key());
                Ok(pg_result)
            }
            Ok(Err(e)) => Err(DSError::DatabaseCustom(format!("Upsert failed: {e}"))),
            Err(_) => Err(DSError::DatabaseCustom("Worker dropped".to_string())),
        }
    }
}

pub trait DbEntityVecExt<T: DbEntity> {
    async fn upsert_all(
        &self,
        app_context: &AppContext,
        db_context: &DbContext,
    ) -> (Vec<Option<PgQueryResult>>, usize);
}
impl<T: DbEntity> DbEntityVecExt<T> for Vec<T> {
    #[tracing::instrument(skip(self, app_context, db_context))]
    async fn upsert_all(
        &self,
        app_context: &AppContext,
        db_context: &DbContext,
    ) -> (Vec<Option<PgQueryResult>>, usize) {
        if self.is_empty() {
            return (Vec::new(), 0);
        }

        let items: Vec<T> = self.clone();
        let type_name = T::type_name();

        tracing::debug!("Upserting {} `{type_name}`s to database", items.len());
        let mut receivers = Vec::new();

        let pb = app_context
            .progress_reporter_mode
            .create_reporter(None, "Dispatching upsert queries...");
        for item in items {
            if db_context.key_cache.contains(&item.cache_key()) {
                pb.inc(1);
                continue;
            }

            let (result_tx, result_rx) = tokio::sync::oneshot::channel::<SqlxJobResult>();
            let query = item.upsert_query();
            let job = SqlxJob { query, result_tx };

            if let Err(e) = db_context.sqlx_tx.send(SqlxJobOrFlush::Job(job)).await {
                tracing::warn!("Failed to send job: {e:?}");
                continue;
            }

            receivers.push((item, result_rx));
            pb.inc(1);
        }
        pb.finish();

        let _ = db_context.sqlx_tx.send(SqlxJobOrFlush::Flush).await;

        // Wait for all results in parallel
        let pb = app_context
            .progress_reporter_mode
            .create_reporter(None, "Awaiting upsert results...");
        let results: Vec<_> = stream::iter(receivers)
            .map(|(item, rx)| async move {
                match rx.await {
                    Ok(Ok(pg_result)) => {
                        db_context.key_cache.insert(item.cache_key());
                        Ok(Some(pg_result))
                    }
                    Ok(Err(e)) => Err(DSError::DatabaseCustom(format!("Upsert failed: {e}"))),
                    Err(_) => Err(DSError::DatabaseCustom("Worker dropped".to_string())),
                }
            })
            .buffer_unordered(app_context.config.db_concurrency_limit)
            .collect()
            .await;
        pb.finish();

        let (successes, failed_count) = partition_and_track_errors(
            results,
            db_context,
            &format!("Database upsert failures for `{type_name}`"),
        )
        .await;

        (successes, failed_count)
    }
}

#[async_trait]
pub trait PrimaryKey:
    Debug + Eq + Hash + Send + Sync + Unpin + for<'a> FromRow<'a, sqlx::postgres::PgRow>
{
    type Entity: DbEntity<Pk = Self>;

    fn create_select_query(&self) -> StaticPgQuery;

    fn cache_key(&self) -> CacheKey;

    async fn verify_by_key(self, db_context: &DbContext) -> Result<Option<Self>, DSError> {
        Self::Entity::verify_by_key(db_context, self).await
    }
}
