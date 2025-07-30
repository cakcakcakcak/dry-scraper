use async_trait::async_trait;

use super::{DbContext, DbPool, StaticPgQuery};
use crate::lp_error::LPError;

#[async_trait]
pub trait Persistable: std::fmt::Debug + Sized + Clone + Send + Sync + 'static {
    type Id: Send + Sync;

    fn id(&self) -> Self::Id;

    fn create_upsert_query(&self) -> StaticPgQuery;

    async fn try_db(db_context: &DbContext, id: Self::Id) -> Result<Option<Self>, LPError>;

    async fn verify_relationships(
        &self,
        _context: &DbContext,
    ) -> Result<RelationshipIntegrity, LPError> {
        Ok(RelationshipIntegrity::AllValid)
    }

    #[tracing::instrument(skip(db_context))]
    async fn upsert(&self, db_context: &DbContext) -> Result<(), LPError> {
        let pool = db_context.pool.clone();
        let (result_tx, result_rx) = tokio::sync::oneshot::channel();

        // Clone self for the closure
        let self_clone = self.clone();

        let job = Box::pin(async move {
            crate::util::sqlx_operation_with_retries(|| async {
                // Create a fresh query for each retry
                let query = self_clone.create_upsert_query();
                query.execute(&pool).await.map(|_| ())
            })
            .await
        });

        db_context
            .sqlx_tx
            .send((job, result_tx))
            .await
            .map_err(|e| LPError::DatabaseCustom(format!("Worker channel send failed: {e}")))?;

        match result_rx.await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(LPError::DatabaseCustom(format!("Upsert failed: {e}"))),
            Err(_) => Err(LPError::DatabaseCustom("Worker dropped".to_string())),
        }
    }
}

#[derive(Debug)]
pub enum RelationshipIntegrity {
    AllValid,
    Missing(Vec<PrimaryKey>),
}

#[derive(Debug)]
pub enum PrimaryKey {
    ApiCache {
        endpoint: String,
    },
    NhlSeason {
        id: i32,
    },
    NhlFranchise {
        id: i32,
    },
    NhlTeam {
        id: i32,
    },
    NhlPlayer {
        id: i32,
    },
    NhlGame {
        id: i32,
    },
    NhlPlayoffSeries {
        season_id: i32,
        series_letter: String,
    },
    NhlPlay {
        game_id: i32,
        event_id: i32,
    },
    NhlRosterSpot {
        game_id: i32,
        player_id: i32,
    },
}
