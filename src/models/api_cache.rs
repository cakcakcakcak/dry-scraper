use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::db::{DbContext, Persistable, PrimaryKey, StaticPgQuery};
use crate::lp_error::LPError;

use crate::bind;
use crate::sqlx_operation_with_retries;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApiCache {
    pub endpoint: String,
    pub raw_data: String,
    pub last_updated: Option<chrono::NaiveDateTime>,
}
#[async_trait]
impl Persistable for ApiCache {
    type Pk = ApiCacheKey;

    fn id(&self) -> Self::Pk {
        Self::Pk {
            endpoint: self.endpoint.clone(),
        }
    }

    #[tracing::instrument(skip(db_context))]
    async fn fetch_from_db(db_context: &DbContext, id: &Self::Pk) -> Result<Option<Self>, LPError> {
        match sqlx_operation_with_retries!(
            sqlx::query_as::<_, Self>(r#"SELECT * FROM api_cache WHERE endpoint=$1"#)
                .bind(&id.endpoint.clone())
                .fetch_optional(&db_context.pool)
                .await
        )
        .await
        {
            Ok(Some(record)) => {
                tracing::debug!(
                    "Record found in lp table api_cache for endpoint {}",
                    id.endpoint
                );
                Ok(Some(record))
            }
            Ok(None) => {
                tracing::debug!(
                    "Record NOT found in lp table api_cache for endpoint {}",
                    id.endpoint
                );
                Ok(None)
            }
            Err(e) => {
                tracing::warn!(
                    "Error encountered while querying api_cache for endpoint {}",
                    id.endpoint
                );
                Err(LPError::Database(e))
            }
        }
    }

    fn create_upsert_query(&self) -> StaticPgQuery {
        bind!(
            sqlx::query(
                r#"INSERT INTO api_cache (endpoint, raw_data)
                                    VALUES ($1,$2)
                                    ON CONFLICT (endpoint) DO UPDATE SET 
                                        endpoint = EXCLUDED.endpoint,
                                        raw_data = EXCLUDED.raw_data,
                                        last_updated = now()
                                    "#,
            ),
            self.endpoint,
            self.raw_data,
        )
    }
}

#[derive(Debug)]
pub struct ApiCacheKey {
    pub endpoint: String,
}
impl PrimaryKey for ApiCacheKey {
    fn create_select_query(&self) -> StaticPgQuery {
        sqlx::query("SELECT * FROM api_cache WHERE endpoint=$1").bind(self.endpoint.clone())
    }
}
