use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;

use crate::sqlx_operation_with_retries;

#[derive(Debug, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct NhlTeam {
    pub id: i32,
    pub franchise_id: Option<i32>,
    pub full_name: String,
    pub league_id: i32,
    pub raw_tricode: String,
    #[serde(rename = "triCode")]
    pub tricode: String,
    pub api_cache_endpoint: Option<String>,
    pub raw_json: Option<serde_json::Value>,
    pub last_updated: Option<chrono::NaiveDateTime>,
}
impl NhlTeam {
    pub async fn upsert(&self, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), sqlx::Error> {
        sqlx_operation_with_retries!(
            sqlx::query(
                r#"INSERT INTO nhl_team (
                                        id, 
                                        franchise_id, 
                                        full_name, 
                                        league_id, 
                                        raw_tricode, 
                                        tricode,
                                        raw_json,
                                        api_cache_endpoint
                                    ) VALUES (
                                        $1,$2,$3,$4,$5,$6,$7,$8)
                                    ON CONFLICT (id) DO UPDATE SET 
                                        franchise_id = EXCLUDED.franchise_id,
                                        full_name = EXCLUDED.full_name, 
                                        league_id = EXCLUDED.league_id,
                                        raw_tricode = EXCLUDED.raw_tricode,
                                        tricode = EXCLUDED.tricode,
                                        raw_json = EXCLUDED.raw_json,
                                        api_cache_endpoint = EXCLUDED.api_cache_endpoint,
                                        last_updated = now()
                                    "#
            )
            .bind(self.id)
            .bind(&self.franchise_id)
            .bind(&self.full_name)
            .bind(&self.league_id)
            .bind(&self.raw_tricode)
            .bind(&self.tricode)
            .bind(&self.raw_json)
            .bind(&self.api_cache_endpoint)
            .execute(pool)
            .await
        )
        .await?;
        Ok(())
    }
}
