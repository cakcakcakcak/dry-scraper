use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;

use crate::db::{DbPool, Persistable};
use crate::lp_error::LPError;
use crate::models::traits::{DbStruct, IntoDbStruct};

use crate::impl_has_type_name;
use crate::sqlx_operation_with_retries;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NhlTeamJson {
    pub id: i32,
    pub franchise_id: Option<i32>,
    pub full_name: String,
    pub league_id: i32,
    pub raw_tricode: String,
    #[serde(rename = "triCode")]
    pub tricode: String,
}
impl IntoDbStruct for NhlTeamJson {
    type U = NhlTeam;

    fn to_db_struct(self) -> Self::U {
        let NhlTeamJson {
            id,
            franchise_id,
            full_name,
            league_id,
            raw_tricode,
            tricode,
        } = self;
        NhlTeam {
            id,
            franchise_id,
            full_name,
            league_id,
            raw_tricode,
            tricode,
            endpoint: String::new(),
            raw_json: serde_json::Value::Null,
            last_updated: None,
        }
    }
}
#[derive(Debug, FromRow, Clone)]
pub struct NhlTeam {
    pub id: i32,
    pub franchise_id: Option<i32>,
    pub full_name: String,
    pub league_id: i32,
    pub raw_tricode: String,
    pub tricode: String,
    pub endpoint: String,
    pub raw_json: serde_json::Value,
    pub last_updated: Option<chrono::NaiveDateTime>,
}
impl DbStruct for NhlTeam {
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
#[async_trait]
impl Persistable for NhlTeam {
    type Id = i32;

    fn id(&self) -> Self::Id {
        self.id
    }

    #[tracing::instrument(skip(pool))]
    async fn try_db(pool: &DbPool, id: Self::Id) -> Result<Option<Self>, LPError> {
        sqlx_operation_with_retries!(
            sqlx::query_as::<_, Self>(r#"SELECT * FROM nhl_season WHERE id=$1"#)
                .bind(id)
                .fetch_optional(pool)
                .await
        )
        .await
        .map_err(LPError::from)
    }

    fn create_query(&self) -> sqlx::query::Query<'_, sqlx::Postgres, sqlx::postgres::PgArguments> {
        sqlx::query(
            r#"INSERT INTO nhl_team (
                                        id, 
                                        franchise_id, 
                                        full_name, 
                                        league_id, 
                                        raw_tricode, 
                                        tricode,
                                        raw_json,
                                        endpoint
                                    ) VALUES (
                                        $1,$2,$3,$4,$5,$6,$7,$8)
                                    ON CONFLICT (id) DO UPDATE SET 
                                        franchise_id = EXCLUDED.franchise_id,
                                        full_name = EXCLUDED.full_name, 
                                        league_id = EXCLUDED.league_id,
                                        raw_tricode = EXCLUDED.raw_tricode,
                                        tricode = EXCLUDED.tricode,
                                        raw_json = EXCLUDED.raw_json,
                                        endpoint = EXCLUDED.endpoint,
                                        last_updated = now()
                                    "#,
        )
        .bind(&self.id)
        .bind(&self.franchise_id)
        .bind(&self.full_name)
        .bind(&self.league_id)
        .bind(&self.raw_tricode)
        .bind(&self.tricode)
        .bind(&self.raw_json)
        .bind(&self.endpoint)
    }
}

impl_has_type_name!(NhlTeamJson);
impl_has_type_name!(NhlTeam);
