use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;

use crate::api::cacheable_api::CacheableApi;
use crate::api::nhl::nhl_stats_api::NhlStatsApi;
use crate::db::{DbPool, Persistable};
use crate::lp_error::LPError;
use crate::models::traits::{DbStruct, IntoDbStruct};

use crate::impl_has_type_name;
use crate::sqlx_operation_with_retries;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NhlFranchiseJson {
    pub id: i32,
    pub full_name: String,
    pub team_common_name: String,
    pub team_place_name: String,
}
impl IntoDbStruct for NhlFranchiseJson {
    type U = NhlFranchise;

    fn to_db_struct(self) -> Self::U {
        let NhlFranchiseJson {
            id,
            full_name,
            team_common_name,
            team_place_name,
        } = self;
        NhlFranchise {
            id,
            full_name,
            team_common_name,
            team_place_name,
            endpoint: String::new(),
            raw_json: serde_json::Value::Null,
            last_updated: None,
        }
    }
}

#[derive(Clone, Debug, FromRow)]
pub struct NhlFranchise {
    pub id: i32,
    pub full_name: String,
    pub team_common_name: String,
    pub team_place_name: String,
    pub raw_json: serde_json::Value,
    pub endpoint: String,
    pub last_updated: Option<chrono::NaiveDateTime>,
}
impl DbStruct for NhlFranchise {
    #[tracing::instrument]
    fn fill_context(&mut self, endpoint: String, raw_data: String) {
        self.raw_json = match serde_json::from_str(&raw_data) {
            Ok(value) => value,
            Err(e) => {
                tracing::warn!(
                    endpoint,
                    "Failed to parse `raw_data` into `serde_json::Value`: {e}"
                );
                serde_json::Value::Null
            }
        };
        self.endpoint = endpoint;
    }
}
impl NhlFranchise {
    pub async fn verify_relationships(
        &self,
        nhl_stats_api: &NhlStatsApi,
        pool: &DbPool,
    ) -> Result<(), LPError> {
        let _ = nhl_stats_api
            .get_or_cache_endpoint(pool, &self.endpoint)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl Persistable for NhlFranchise {
    type Id = i32;

    fn id(&self) -> Self::Id {
        self.id
    }

    #[tracing::instrument(skip(pool))]
    async fn try_db(pool: &DbPool, id: Self::Id) -> Result<Option<Self>, LPError> {
        sqlx_operation_with_retries!(
            sqlx::query_as::<_, NhlFranchise>(r#"SELECT * FROM nhl_franchise WHERE id=$1"#)
                .bind(id)
                .fetch_optional(pool)
                .await
        )
        .await
        .map_err(LPError::from)
    }

    fn create_query(&self) -> sqlx::query::Query<'_, sqlx::Postgres, sqlx::postgres::PgArguments> {
        sqlx::query(
            r#"INSERT INTO nhl_franchise (
                                        id, 
                                        full_name, 
                                        team_common_name, 
                                        team_place_name,
                                        raw_json,
                                        endpoint
                                    ) VALUES (
                                        $1,$2,$3,$4,$5,$6)
                                    ON CONFLICT (id) DO UPDATE SET 
                                        full_name = EXCLUDED.full_name,
                                        team_common_name = EXCLUDED.team_common_name, 
                                        team_place_name = EXCLUDED.team_place_name,
                                        raw_json = EXCLUDED.raw_json,
                                        endpoint = EXCLUDED.endpoint,
                                        last_updated = now()
                                    "#,
        )
        .bind(&self.id)
        .bind(&self.full_name)
        .bind(&self.team_common_name)
        .bind(&self.team_place_name)
        .bind(&self.raw_json)
        .bind(&self.endpoint)
    }
}

impl_has_type_name!(NhlFranchiseJson);
impl_has_type_name!(NhlFranchise);
