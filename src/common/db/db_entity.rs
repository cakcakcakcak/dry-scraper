use std::fmt::Debug;

use async_trait::async_trait;
use sqlx::{FromRow, postgres::PgQueryResult};

use crate::{
    common::{
        api::CacheableApi,
        db::{DbContext, DbPool, SqlxJobResult, StaticPgQuery},
        errors::LPError,
    },
    sqlx_operation_with_retries,
};

#[async_trait]
pub trait DbEntity:
    Debug + Sized + Clone + Send + Sync + for<'a> FromRow<'a, sqlx::postgres::PgRow> + 'static
{
    type Pk: PrimaryKey;

    fn id(&self) -> Self::Pk;

    fn create_upsert_query(&self) -> StaticPgQuery;

    #[tracing::instrument(skip(db_context))]
    async fn verify_by_key(
        db_context: &DbContext,
        id: Self::Pk,
    ) -> Result<Option<Self::Pk>, LPError> {
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
                tracing::debug!("Record found in lp database for key {:?}", id);
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

    async fn upsert_and_fix_relationships(
        &self,
        db_context: &DbContext,
        api: &<Self::Pk as PrimaryKey>::Api,
    ) -> Result<PgQueryResult, LPError> {
        match self.verify_relationships(db_context).await? {
            RelationshipIntegrity::AllValid => (),
            RelationshipIntegrity::Missing(keys) => {
                for key in keys {
                    key.upsert_from_api(db_context, api).await?
                }
            }
        }

        self.upsert(db_context).await
    }

    #[tracing::instrument(skip(self, db_context))]
    async fn upsert(&self, db_context: &DbContext) -> Result<PgQueryResult, LPError> {
        let pool: DbPool = db_context.pool.clone();

        let (result_tx, result_rx) = tokio::sync::oneshot::channel();

        let self_clone: Self = self.clone();

        let job = Box::pin(async move {
            crate::common::util::sqlx_operation_with_retries(|| async {
                let query: StaticPgQuery = self_clone.create_upsert_query();
                tracing::debug!("Attempting upsert for {:?}", self_clone.id());

                let res: SqlxJobResult = match query.execute(&pool).await {
                    Ok(pg_result) => {
                        tracing::debug!(
                            "Upsert for {:?} affected {:?}",
                            self_clone.id(),
                            pg_result
                        );
                        Ok(pg_result)
                    }
                    Err(e) => {
                        tracing::warn!("Upsert attempt for {:?} failed: {:?}", self_clone.id(), e);
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
            Ok(Ok(pg_result)) => Ok(pg_result),
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
pub trait PrimaryKey: Debug + Send + Sync {
    type Api: CacheableApi + Send + Sync;

    fn create_select_query(&self) -> StaticPgQuery;

    async fn upsert_from_api(&self, db_context: &DbContext, api: &Self::Api)
    -> Result<(), LPError>;
}
