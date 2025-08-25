use std::fmt::Debug;

use async_trait::async_trait;
use sqlx::{FromRow, postgres::PgQueryResult};

use crate::{
    any_primary_key::AnyPrimaryKey,
    common::{
        api::CacheableApi,
        db::{DbContext, DbPool, SqlxJobResult, StaticPgQuery, StaticPgQueryAs},
        errors::LPError,
        models::traits::HasTypeName,
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
    type Pk: PrimaryKey;

    fn pk(&self) -> Self::Pk;

    fn any_pk(&self) -> AnyPrimaryKey {
        self.pk().any_pk()
    }

    fn fmt_debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.pk(), f)
    }

    async fn warm_key_cache(db_context: &DbContext) -> Result<(), LPError> {
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
    ) -> Result<Option<Self::Pk>, LPError> {
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
    ) -> Result<Option<Self>, LPError> {
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
                Self::from_row(&row).map(Some).map_err(LPError::from)
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
                Err(LPError::from(e))
            }
        }
    }

    #[tracing::instrument(skip(_db_context, self))]
    async fn verify_relationships(
        &self,
        _db_context: &DbContext,
    ) -> Result<RelationshipIntegrity<Self::Pk>, LPError> {
        Ok(RelationshipIntegrity::AllValid)
    }

    #[tracing::instrument(skip(db_context, api))]
    async fn fix_relationships_and_upsert(
        &self,
        db_context: &DbContext,
        api: &<Self::Pk as PrimaryKey>::Api,
    ) -> Result<Option<PgQueryResult>, LPError> {
        match self.verify_relationships(db_context).await? {
            RelationshipIntegrity::AllValid => (),
            RelationshipIntegrity::Missing(keys) => {
                for key in keys {
                    key.upsert_from_api(db_context, api).await?
                }
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
    async fn upsert(&self, db_context: &DbContext) -> Result<PgQueryResult, LPError> {
        let pool: DbPool = db_context.pool.clone();

        let (result_tx, result_rx) = tokio::sync::oneshot::channel();

        let self_clone: Self = self.clone();

        let job = Box::pin(async move {
            crate::common::util::sqlx_operation_with_retries(|| async {
                let query: StaticPgQuery = self_clone.upsert_query();
                tracing::debug!("Attempting upsert for {:?}", self_clone.pk());

                let res: SqlxJobResult = match query.execute(&pool).await {
                    Ok(pg_result) => {
                        tracing::debug!(
                            "Upsert for {:?} affected {:?}",
                            self_clone.pk(),
                            pg_result
                        );
                        Ok(pg_result)
                    }
                    Err(e) => {
                        tracing::warn!("Upsert attempt for {:?} failed: {:?}", self_clone.pk(), e);
                        Err(e)
                    }
                };
                res
            })
            .await
        });

        db_context
            .sqlx_tx
            .send((job, result_tx))
            .await
            .map_err(|e| LPError::DatabaseCustom(format!("Worker channel send failed: {e}")))?;

        match result_rx.await {
            Ok(Ok(pg_result)) => {
                db_context.key_cache.insert(self.any_pk());
                Ok(pg_result)
            }
            Ok(Err(e)) => Err(LPError::DatabaseCustom(format!("Upsert failed: {e}"))),
            Err(_) => Err(LPError::DatabaseCustom("Worker dropped".to_string())),
        }
    }
}

#[derive(Debug)]
pub enum RelationshipIntegrity<Pk: PrimaryKey> {
    AllValid,
    Missing(Vec<Pk>),
}

#[async_trait]
pub trait PrimaryKey:
    Debug + Send + Sync + Unpin + for<'a> FromRow<'a, sqlx::postgres::PgRow>
{
    type Api: CacheableApi + Send + Sync;

    fn any_pk(&self) -> AnyPrimaryKey;

    fn create_select_query(&self) -> StaticPgQuery;

    async fn upsert_from_api(&self, db_context: &DbContext, api: &Self::Api)
    -> Result<(), LPError>;
}
