use async_trait::async_trait;
use sqlx::FromRow;

use super::{DbContext, DbPool, SqlxJobResult, StaticPgQuery};
use crate::lp_error::LPError;

use crate::sqlx_operation_with_retries;

#[async_trait]
pub trait Persistable:
    std::fmt::Debug + Sized + Clone + Send + Sync + for<'a> FromRow<'a, sqlx::postgres::PgRow> + 'static
{
    type Id: PrimaryKey;

    fn id(&self) -> Self::Id;

    fn create_upsert_query(&self) -> StaticPgQuery;

    #[tracing::instrument(skip(db_context))]
    async fn fetch_from_db(db_context: &DbContext, id: &Self::Id) -> Result<Option<Self>, LPError> {
        match sqlx_operation_with_retries!(
            id.create_select_query()
                .fetch_optional(&db_context.pool)
                .await
        )
        .await
        {
            Ok(Some(row)) => Self::from_row(&row).map(Some).map_err(LPError::from),
            Ok(None) => Ok(None),
            Err(e) => Err(LPError::from(e)),
        }
    }

    async fn verify_relationships(
        &self,
        _context: &DbContext,
    ) -> Result<RelationshipIntegrity, LPError> {
        Ok(RelationshipIntegrity::AllValid)
    }

    #[tracing::instrument(skip(db_context))]
    async fn upsert(
        &self,
        db_context: &DbContext,
    ) -> Result<sqlx::postgres::PgQueryResult, LPError> {
        let pool: DbPool = db_context.pool.clone();
        let (result_tx, result_rx) = tokio::sync::oneshot::channel();

        let self_clone: Self = self.clone();

        let job = Box::pin(async move {
            crate::util::sqlx_operation_with_retries(|| async {
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
pub enum RelationshipIntegrity {
    AllValid,
    Missing(Vec<Box<dyn PrimaryKey>>),
}

pub trait PrimaryKey: std::fmt::Debug + Send + Sync {
    fn create_select_query(&self) -> StaticPgQuery;
}
