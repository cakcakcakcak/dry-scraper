use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::FromRow;

use crate::LPError;
use crate::api::CacheableApi;
use crate::api::nhl::NhlStatsApi;
use crate::db::{DbContext, Persistable, PrimaryKey, StaticPgQuery};
use crate::models::nhl::DefaultNhlContext;
use crate::models::traits::{DbStruct, IntoDbStruct};

use crate::bind;
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
    type DbStruct = NhlFranchise;
    type Context = DefaultNhlContext;

    fn to_db_struct(self, context: Self::Context) -> Self::DbStruct {
        let NhlFranchiseJson {
            id,
            full_name,
            team_common_name,
            team_place_name,
        } = self;
        let DefaultNhlContext { endpoint, raw_json } = context;
        NhlFranchise {
            id,
            full_name,
            team_common_name,
            team_place_name,
            endpoint,
            raw_json,
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
    type IntoDbStruct = NhlFranchiseJson;
}

#[async_trait]
impl Persistable for NhlFranchise {
    type Id = PrimaryKey;
    fn id(&self) -> Self::Id {
        PrimaryKey::NhlFranchise { id: self.id }
    }

    #[tracing::instrument(skip(db_context))]
    async fn try_db(db_context: &DbContext, id: Self::Id) -> Result<Option<Self>, LPError> {
        match id {
            PrimaryKey::NhlFranchise { id } => sqlx_operation_with_retries!(
                sqlx::query_as::<_, NhlFranchise>(r#"SELECT * FROM nhl_franchise WHERE id=$1"#)
                    .bind(id.clone())
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
            ),
            self.id,
            self.full_name,
            self.team_common_name,
            self.team_place_name,
            self.raw_json,
            self.endpoint,
        )
    }
}

impl_has_type_name!(NhlFranchiseJson);
impl_has_type_name!(NhlFranchise);
