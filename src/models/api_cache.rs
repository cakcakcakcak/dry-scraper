use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::sqlx_operation_with_retries;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ApiCache {
    pub endpoint: String,
    pub raw_data: String,
    pub last_updated: Option<chrono::NaiveDateTime>,
}

impl ApiCache {
    pub async fn upsert(&self, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), sqlx::Error> {
        sqlx_operation_with_retries!(
            sqlx::query(
                r#"INSERT INTO api_cache (endpoint, raw_data)
                                    VALUES ($1,$2)
                                    ON CONFLICT (endpoint) DO UPDATE SET 
                                        endpoint = EXCLUDED.endpoint,
                                        raw_data = EXCLUDED.raw_data,
                                        last_updated = now()
                                    "#
            )
            .bind(&self.endpoint)
            .bind(&self.raw_data)
            .execute(pool)
            .await
        )
        .await?;
        Ok(())
    }
}
