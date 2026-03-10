#![allow(async_fn_in_trait)]

use futures::stream::{self, StreamExt};
use std::{
    cmp::{min, Eq},
    collections::HashSet,
    fmt::Debug,
    hash::Hash,
};

use async_trait::async_trait;
use sqlx::{postgres::PgQueryResult, FromRow};

use crate::{
    any_primary_key::AnyPrimaryKey,
    common::{
        api::CacheableApi,
        app_context::AppContext,
        db::{DbContext, SqlxJob, SqlxJobOrFlush, SqlxJobResult, StaticPgQuery, StaticPgQueryAs},
        errors::DSError,
        models::traits::HasTypeName,
        util::track_and_filter_errors,
    },
    config::CONFIG,
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
    type Pk: PrimaryKey;

    fn pk(&self) -> Self::Pk;

    fn any_pk(&self) -> AnyPrimaryKey {
        self.pk().any_pk()
    }

    fn foreign_keys(&self) -> Vec<Self::Pk> {
        vec![]
    }

    fn fmt_debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.pk(), f)
    }

    async fn warm_key_cache(db_context: &DbContext) -> Result<(), DSError> {
        let select_query: StaticPgQueryAs<Self::Pk> = Self::select_key_query();
        let key_vec: Vec<Self::Pk> = select_query.fetch_all(&db_context.pool).await?;
        for entity in key_vec {
            db_context.key_cache.insert(entity.any_pk());
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
        if db_context.key_cache.contains(&id.any_pk()) {
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
            id.create_select_query()
                .fetch_optional(&db_context.pool)
                .await
        )
        .await
        {
            Ok(Some(row)) => {
                tracing::debug!(
                    "Record found in lp database for key {:?}. Adding key to key cache",
                    id
                );
                db_context.key_cache.insert(id.any_pk());
                Self::from_row(&row).map(Some).map_err(DSError::from)
            }
            Ok(None) => {
                tracing::debug!("Record not found in lp database for key {:?}", id);
                Ok(None)
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to fetch from lp database using key {:?}: {:?}",
                    id,
                    e
                );
                Err(DSError::from(e))
            }
        }
    }

    #[tracing::instrument(skip(db_context))]
    async fn verify_relationships(
        &self,
        db_context: &DbContext,
    ) -> Result<RelationshipIntegrity<Self::Pk>, DSError> {
        self.foreign_keys().verify_keys(db_context).await
    }

    #[tracing::instrument(skip(db_context, api))]
    async fn fix_relationships_and_upsert(
        &self,
        db_context: &DbContext,
        api: &<Self::Pk as PrimaryKey>::Api,
    ) -> Result<Option<PgQueryResult>, DSError> {
        match self.verify_relationships(db_context).await? {
            RelationshipIntegrity::AllValid => (),
            RelationshipIntegrity::Missing(keys) => {
                stream::iter(keys)
                    .map(|key| async move { key.upsert_from_api(db_context, api).await })
                    .buffer_unordered(CONFIG.db_concurrency_limit)
                    .collect::<Vec<_>>()
                    .await;
            }
        }

        if db_context.key_cache.contains(&self.any_pk()) {
            tracing::debug!("Key cache contains {:?}, skipping upsert.", self.pk());
            return Ok(None);
        }

        match self.upsert(db_context).await {
            Ok(query_result) => Ok(Some(query_result)),
            Err(e) => Err(e),
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
                db_context.key_cache.insert(self.any_pk());
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
        api: &<<T as DbEntity>::Pk as PrimaryKey>::Api,
    ) -> Vec<Option<PgQueryResult>>;
}
impl<T: DbEntity> DbEntityVecExt<T> for Vec<T> {
    #[tracing::instrument(skip(self, app_context, db_context, api))]
    async fn upsert_all(
        &self,
        app_context: &AppContext,
        db_context: &DbContext,
        api: &<<T as DbEntity>::Pk as PrimaryKey>::Api,
    ) -> Vec<Option<PgQueryResult>> {
        if self.is_empty() {
            tracing::debug!("No items to upsert, returning early");
            return Vec::new();
        }

        let items: Vec<T> = self.clone();
        let type_name = T::type_name();

        tracing::debug!(
            "Determining missing foreign keys for {} `{type_name}`s.",
            items.len()
        );
        let mut missing_keys: HashSet<<T as DbEntity>::Pk> = HashSet::new();
        for item in &items {
            if db_context.key_cache.contains(&item.any_pk()) {
                continue;
            }
            for fk in item.foreign_keys() {
                if !db_context.key_cache.contains(&fk.any_pk()) {
                    missing_keys.insert(fk);
                }
            }
        }

        let missing_keys: Vec<<T as DbEntity>::Pk> = missing_keys.into_iter().collect();
        tracing::debug!(
            "Collected {} unique foreign keys to verify",
            missing_keys.len()
        );

        tracing::debug!("Upserting {} missing foreign keys", missing_keys.len());
        stream::iter(missing_keys)
            .map(|key| async move { key.upsert_from_api(db_context, api).await })
            .buffer_unordered(min(
                CONFIG.api_concurrency_limit,
                CONFIG.db_concurrency_limit,
            ))
            .collect::<Vec<_>>()
            .await;

        tracing::debug!(
            "Phase 4: Upserting {} `{type_name}`s to lp database",
            items.len()
        );
        let mut receivers = Vec::new();

        let pb = app_context
            .progress_reporter_mode
            .create_reporter(None, "Dispatching upsert queries...");
        for item in items {
            if db_context.key_cache.contains(&item.any_pk()) {
                tracing::debug!("Key cache contains {:?}, skipping upsert.", item.pk());
                pb.inc(1);
                continue;
            }
            for fk in item.foreign_keys() {
                if !db_context.key_cache.contains(&fk.any_pk()) {
                    continue;
                }
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

        // 3. Then wait for all results in parallel
        let pb = app_context
            .progress_reporter_mode
            .create_reporter(None, "Awaiting upsert results...");
        let results: Vec<_> = stream::iter(receivers)
            .map(|(item, rx)| async move {
                match rx.await {
                    Ok(Ok(pg_result)) => {
                        db_context.key_cache.insert(item.any_pk());
                        Ok(Some(pg_result))
                    }
                    Ok(Err(e)) => Err(DSError::DatabaseCustom(format!("Upsert failed: {e}"))),
                    Err(_) => Err(DSError::DatabaseCustom("Worker dropped".to_string())),
                }
            })
            .buffer_unordered(CONFIG.db_concurrency_limit)
            .collect()
            .await;
        pb.finish();

        track_and_filter_errors(results, db_context).await
    }
}

#[derive(Debug)]
pub enum RelationshipIntegrity<Pk: PrimaryKey> {
    AllValid,
    Missing(Vec<Pk>),
}

#[async_trait]
pub trait PrimaryKey:
    Debug + Eq + Hash + Send + Sync + Unpin + for<'a> FromRow<'a, sqlx::postgres::PgRow>
{
    type Api: CacheableApi + Send + Sync;

    fn any_pk(&self) -> AnyPrimaryKey;

    fn create_select_query(&self) -> StaticPgQuery;

    async fn upsert_from_api(&self, db_context: &DbContext, api: &Self::Api)
        -> Result<(), DSError>;

    async fn verify_by_key(self, db_context: &DbContext) -> Result<Option<Self>, DSError>;
}

#[async_trait]
pub trait PrimaryKeyExt<K: PrimaryKey> {
    async fn verify_keys(self, db_context: &DbContext)
        -> Result<RelationshipIntegrity<K>, DSError>;
}
#[async_trait]
impl<K: PrimaryKey> PrimaryKeyExt<K> for Vec<K> {
    async fn verify_keys(
        self,
        db_context: &DbContext,
    ) -> Result<RelationshipIntegrity<K>, DSError> {
        let mut missing: Vec<K> = vec![];

        let results = stream::iter(self)
            .map(|fk| async move { fk.verify_by_key(db_context).await })
            .buffer_unordered(CONFIG.db_concurrency_limit)
            .collect::<Vec<_>>()
            .await;
        for result in results {
            if let Ok(Some(pk)) = result {
                missing.push(pk)
            }
        }

        match missing.len() {
            0 => Ok(RelationshipIntegrity::AllValid),
            _ => Ok(RelationshipIntegrity::Missing(missing)),
        }
    }
}
