use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;

use crate::sqlx_operation_with_retries;

#[derive(Debug, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct NhlFranchise {
    pub id: i32,
    pub full_name: String,
    pub team_common_name: String,
    pub team_place_name: String,
    pub raw_json: Option<serde_json::Value>,
    pub api_cache_endpoint: Option<String>,
    pub last_updated: Option<chrono::NaiveDateTime>,
}
impl NhlFranchise {
    pub async fn upsert(&self, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), sqlx::Error> {
        sqlx_operation_with_retries!(
            sqlx::query(
                r#"INSERT INTO nhl_franchise (
                                        id, 
                                        full_name, 
                                        team_common_name, 
                                        team_place_name,
                                        raw_json,
                                        api_cache_endpoint
                                    ) VALUES (
                                        $1,$2,$3,$4,$5,$6)
                                    ON CONFLICT (id) DO UPDATE SET 
                                        full_name = EXCLUDED.full_name,
                                        team_common_name = EXCLUDED.team_common_name, 
                                        team_place_name = EXCLUDED.team_place_name,
                                        raw_json = EXCLUDED.raw_json,
                                        api_cache_endpoint = EXCLUDED.api_cache_endpoint,
                                        last_updated = now()
                                    "#
            )
            .bind(self.id)
            .bind(&self.full_name)
            .bind(&self.team_common_name)
            .bind(&self.team_place_name)
            .bind(&self.raw_json)
            .bind(&self.api_cache_endpoint)
            .execute(pool)
            .await
        )
        .await?;
        Ok(())
    }
}
