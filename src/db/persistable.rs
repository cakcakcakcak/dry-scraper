use async_trait::async_trait;

use crate::db::DbPool;
use crate::lp_error::LPError;

use crate::sqlx_operation_with_retries;

#[async_trait]
pub trait Persistable: std::fmt::Debug + Sized {
    type Id: Send + Sync;

    fn id(&self) -> Self::Id;

    fn create_query(&self) -> sqlx::query::Query<'_, sqlx::Postgres, sqlx::postgres::PgArguments>;

    async fn try_db(pool: &DbPool, id: Self::Id) -> Result<Option<Self>, LPError>;

    async fn verify_relationships(&self, _pool: &DbPool) -> Result<RelationshipIntegrity, LPError> {
        Ok(RelationshipIntegrity::AllValid)
    }

    #[tracing::instrument(skip(pool))]
    async fn upsert(&self, pool: &DbPool) -> Result<(), LPError> {
        sqlx_operation_with_retries!(self.create_query().execute(pool).await).await?;
        Ok(())
    }

    async fn upsert_all<T: Send + Persistable>(
        records: Vec<T>,
        pool: &DbPool,
    ) -> Result<(), LPError> {
        let mut tx = pool.begin().await?;

        for record in records {
            if let Err(e) = record.create_query().execute(&mut *tx).await {
                tracing::warn!("Upsert failed for record {record:?}: {e}");
            }
        }

        tx.commit().await?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum RelationshipIntegrity {
    AllValid,
    Missing(Vec<MissingRelationship>),
}

#[derive(Debug)]
pub enum MissingRelationship {
    ApiCache(String),
    NhlSeason(i32),
    NhlFranchise(i32),
    NhlTeam(i32),
    NhlPlayer(i32),
    NhlGame(i32),
    NhlPlayoffSeries(i32, String),
    NhlPlay(i32, i32),
}
