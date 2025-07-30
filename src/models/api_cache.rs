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
    type Id = PrimaryKey;

    fn id(&self) -> Self::Id {
        PrimaryKey::ApiCache {
            endpoint: self.endpoint.clone(),
        }
    }

    #[tracing::instrument(skip(db_context))]
    async fn try_db(db_context: &DbContext, id: Self::Id) -> Result<Option<Self>, LPError> {
        match id {
            PrimaryKey::ApiCache { endpoint } => sqlx_operation_with_retries!(
                sqlx::query_as::<_, Self>(r#"SELECT * FROM api_cache WHERE endpoint=$1"#)
                    .bind(&endpoint.clone())
                    .fetch_optional(&db_context.pool)
                    .await
            )
            .await
            .map_err(LPError::from),
            _ => Err(LPError::DatabaseCustom("Wrong ID variant".to_string())),
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
