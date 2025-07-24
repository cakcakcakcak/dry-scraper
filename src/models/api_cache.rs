use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::db::{DbPool, Persistable};
use crate::lp_error::LPError;

use crate::sqlx_operation_with_retries;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ApiCache {
    pub endpoint: String,
    pub raw_data: String,
    pub last_updated: Option<chrono::NaiveDateTime>,
}
#[async_trait]
impl Persistable for ApiCache {
    type Id = String;

    fn id(&self) -> Self::Id {
        self.endpoint.clone()
    }

    #[tracing::instrument(skip(pool))]
    async fn try_db(pool: &DbPool, id: Self::Id) -> Result<Option<Self>, LPError> {
        sqlx_operation_with_retries!(
            sqlx::query_as::<_, Self>(r#"SELECT * FROM api_cache WHERE id=$1"#)
                .bind(id.clone())
                .fetch_optional(pool)
                .await
        )
        .await
        .map_err(LPError::from)
    }

    fn create_query(&self) -> sqlx::query::Query<'_, sqlx::Postgres, sqlx::postgres::PgArguments> {
        sqlx::query(
            r#"INSERT INTO api_cache (endpoint, raw_data)
                                    VALUES ($1,$2)
                                    ON CONFLICT (endpoint) DO UPDATE SET 
                                        endpoint = EXCLUDED.endpoint,
                                        raw_data = EXCLUDED.raw_data,
                                        last_updated = now()
                                    "#,
        )
        .bind(&self.endpoint)
        .bind(&self.raw_data)
    }
}
